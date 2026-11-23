//! Sprites for logic entities: terrain sub-cells, tanks, bullets, base, power-ups.

use super::{Gfx, player_color, rank_color, z};
use crate::game::*;
use crate::geom::{FIELD, SUB, SUBS};
use crate::rules::Rank;
use crate::terrain::{Terrain, Tile};
use bevy::prelude::*;

/// Terrain sprite per sub-cell, rebuilt each stage (empty until the first sync builds the floor).
#[derive(Resource, Default)]
struct TerrainSprites(Vec<Option<Entity>>);

#[derive(Component)]
struct WaterTile;

/// Shield bubble child entity.
#[derive(Component)]
struct HasRing(Entity);

/// Pixels per terrain sub-cell in the tile textures (8 px/unit * 4 units).
const SUB_PX: f32 = 32.0;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Stage), |mut commands: Commands| commands.insert_resource(TerrainSprites::default())).add_systems(
        Update,
        (sync_terrain, animate_water, attach_tanks, tank_looks, shield_rings, attach_bullets, base_look, powerup_looks)
            .run_if(in_state(Screen::Stage))
            .after(Sim),
    );
}

/// Border plus ground, the 208-unit ground texture stacked up a tall City Run map.
fn spawn_floor(commands: &mut Commands, gfx: &Gfx, height: f32) {
    commands.spawn((
        Sprite { color: Color::srgb(0.22, 0.25, 0.30), custom_size: Some(Vec2::new(FIELD + 4.0, height + 4.0)), ..default() },
        Transform::from_xyz(FIELD / 2.0, height / 2.0, z::GROUND - 0.1),
        DespawnOnExit(Screen::Stage),
    ));
    let mut y = 0.0;
    while y < height {
        let h = (height - y).min(FIELD);
        let px = 4.0; // ground texture pixels per unit
        commands.spawn((
            Sprite {
                image: gfx.ground.clone(),
                custom_size: Some(Vec2::new(FIELD, h)),
                rect: Some(Rect::new(0.0, (FIELD - h) * px, FIELD * px, FIELD * px)),
                ..default()
            },
            Transform::from_xyz(FIELD / 2.0, y + h / 2.0, z::GROUND),
            DespawnOnExit(Screen::Stage),
        ));
        y += h;
    }
}

fn sync_terrain(mut commands: Commands, gfx: Res<Gfx>, terrain: Option<ResMut<Terrain>>, mut sprites: ResMut<TerrainSprites>) {
    let Some(mut terrain) = terrain else { return };
    if sprites.0.is_empty() {
        sprites.0 = vec![None; SUBS * terrain.rows()];
        spawn_floor(&mut commands, &gfx, terrain.height());
    }
    if terrain.dirty.is_empty() {
        return;
    }
    for (x, y) in std::mem::take(&mut terrain.dirty) {
        let i = y * SUBS + x;
        if let Some(old) = sprites.0[i].take() {
            commands.entity(old).try_despawn();
        }
        let plain = Color::WHITE;
        let (image, depth, color) = match terrain.get(x, y) {
            Tile::Empty => continue,
            Tile::Brick | Tile::Secret => (gfx.brick.clone(), z::WALL, plain),
            Tile::Steel => (gfx.steel.clone(), z::WALL, plain),
            Tile::Water => (gfx.water[0].clone(), z::FLOOR, plain),
            Tile::Ice => (gfx.ice.clone(), z::FLOOR, plain),
            Tile::Trees => (gfx.trees.clone(), z::TREES, plain),
            // Fortress: dark plated steel; the gate glows warning orange.
            Tile::Wall => (gfx.steel.clone(), z::WALL, Color::srgb(0.48, 0.45, 0.56)),
            Tile::Gate => (gfx.steel.clone(), z::WALL, Color::srgb(1.25, 0.62, 0.28)),
        };
        // Sample the matching quarter of the 16-unit tile so patterns line up across cells.
        let (qx, qy) = ((x % 4) as f32, (3 - y % 4) as f32);
        let rect = Rect::new(qx * SUB_PX, qy * SUB_PX, (qx + 1.0) * SUB_PX, (qy + 1.0) * SUB_PX);
        let pos = Vec3::new(x as f32 * SUB + SUB / 2.0, y as f32 * SUB + SUB / 2.0, depth);
        let mut e = commands.spawn((
            Sprite { image, color, rect: Some(rect), custom_size: Some(Vec2::splat(SUB)), ..default() },
            Transform::from_translation(pos),
            DespawnOnExit(Screen::Stage),
        ));
        if terrain.get(x, y) == Tile::Water {
            e.insert(WaterTile);
        }
        sprites.0[i] = Some(e.id());
    }
}

fn animate_water(time: Res<Time>, gfx: Res<Gfx>, mut last: Local<usize>, mut water: Query<&mut Sprite, With<WaterTile>>) {
    let frame = (time.elapsed_secs() * 2.5) as usize % 3;
    let changed = water.iter().any(|s| s.image != gfx.water[frame]);
    if frame != *last || changed {
        *last = frame;
        for mut s in &mut water {
            s.image = gfx.water[frame].clone();
        }
    }
}

fn attach_tanks(mut commands: Commands, gfx: Res<Gfx>, added: Query<(Entity, &Tank, &Transform), Added<Tank>>) {
    for (e, tank, tf) in &added {
        let size = tank.size;
        commands.entity(e).insert((Sprite { custom_size: Some(Vec2::splat(size)), ..default() }, Visibility::Hidden)).with_child((
            Sprite {
                image: gfx.dot.clone(),
                color: Color::srgba(0.0, 0.0, 0.0, 0.55),
                custom_size: Some(Vec2::splat(size * 1.3)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, -0.5),
        ));
        super::fx::spawn_sparkle(&mut commands, &gfx, tf.translation.xy());
    }
}

fn tank_looks(
    time: Res<Time>,
    gfx: Res<Gfx>,
    session: Res<Session>,
    run: Res<StageRun>,
    mut tanks: Query<(
        &Tank,
        Option<&Enemy>,
        &mut Sprite,
        &mut Transform,
        &mut Visibility,
        Has<Spawning>,
        Has<Paralyzed>,
        Has<HitFlash>,
        Has<Armour>,
    )>,
) {
    let t = time.elapsed_secs();
    let blink = ((t * 8.0) as u32).is_multiple_of(2);
    for (tank, enemy, mut sprite, mut tf, mut vis, spawning, paralyzed, flash, armour) in &mut tanks {
        vis.set_if_neq(if spawning { Visibility::Hidden } else { Visibility::Inherited });
        tf.translation.z = z::TANK;
        tf.rotation = Quat::from_rotation_z(tank.dir.angle());
        let frame = (tank.odometer / 1.5) as usize % 2;
        let (image, mut color) = match (tank.team, enemy) {
            (Team::Player(i), _) => {
                let level = session.slots[i].as_ref().map_or(0, |s| s.level) as usize;
                (gfx.players[level.min(gfx.players.len() - 1)][frame].clone(), player_color(i))
            }
            (Team::Enemy, Some(e)) => {
                let mut c = rank_color(e.rank, e.kind, e.hp, e.max_hp);
                if e.bonus && blink {
                    c = Color::srgb(1.0, 0.32, 0.28);
                }
                if run.freeze > 0.0 {
                    c = c.mix(&Color::srgb(0.55, 0.8, 1.0), 0.5);
                }
                let image = match e.rank {
                    Rank::Normal => &gfx.enemies[e.kind.index()][frame],
                    Rank::Turret | Rank::Sentry => &gfx.turret,
                    Rank::Mini => &gfx.mini[frame],
                    Rank::Boss => &gfx.boss[frame],
                };
                (image.clone(), c)
            }
            (Team::Enemy, None) => continue,
        };
        if flash {
            color = Color::srgb(2.5, 2.5, 2.5);
        }
        if armour && blink {
            color = Color::srgb(2.0, 2.0, 2.0);
        }
        if paralyzed && blink {
            color = color.with_alpha(0.35);
        }
        if sprite.image != image {
            sprite.image = image;
        }
        sprite.color = color;
    }
}

fn shield_rings(
    mut commands: Commands,
    gfx: Res<Gfx>,
    time: Res<Time>,
    tanks: Query<(Entity, Has<Shield>, Option<&HasRing>), With<Tank>>,
    mut rings: Query<(&mut Transform, &mut Sprite), Without<Tank>>,
) {
    let t = time.elapsed_secs();
    for (e, shielded, ring) in &tanks {
        match (shielded, ring) {
            (true, None) => {
                let child = commands
                    .spawn((
                        Sprite { image: gfx.ring.clone(), custom_size: Some(Vec2::splat(22.0)), ..default() },
                        Transform::from_xyz(0.0, 0.0, 0.6),
                    ))
                    .id();
                commands.entity(e).add_child(child).insert(HasRing(child));
            }
            (false, Some(r)) => {
                commands.entity(r.0).try_despawn();
                commands.entity(e).remove::<HasRing>();
            }
            (true, Some(r)) => {
                if let Ok((mut tf, mut s)) = rings.get_mut(r.0) {
                    tf.scale = Vec3::splat(1.0 + 0.06 * (t * 9.0).sin());
                    tf.rotation = Quat::from_rotation_z(t * 2.0);
                    s.color = Color::linear_rgba(0.6, 1.6, 2.4, 0.55 + 0.25 * (t * 13.0).sin());
                }
            }
            (false, None) => {}
        }
    }
}

fn attach_bullets(mut commands: Commands, gfx: Res<Gfx>, mut added: Query<(Entity, &Bullet, &mut Transform), Added<Bullet>>) {
    for (e, b, mut tf) in &mut added {
        let color = match b.team {
            Team::Player(_) => Color::linear_rgb(3.2, 2.4, 1.0),
            Team::Enemy => Color::linear_rgb(3.4, 0.9, 0.5),
        };
        tf.translation.z = z::BULLET;
        tf.rotation = Quat::from_rotation_z(b.dir.angle());
        commands.entity(e).insert(Sprite { image: gfx.bullet.clone(), color, custom_size: Some(Vec2::new(4.0, 6.0)), ..default() });
    }
}

fn base_look(
    mut commands: Commands,
    gfx: Res<Gfx>,
    run: Res<StageRun>,
    added: Query<Entity, Added<Base>>,
    mut base: Query<(&mut Sprite, &mut Transform), With<Base>>,
) {
    for e in &added {
        commands.entity(e).insert(Sprite { image: gfx.base.clone(), custom_size: Some(Vec2::splat(16.0)), ..default() });
    }
    for (mut s, mut tf) in &mut base {
        tf.translation.z = z::WALL;
        let want = if run.base_alive { &gfx.base } else { &gfx.base_dead };
        if &s.image != want {
            s.image = want.clone();
        }
    }
}

fn powerup_looks(
    mut commands: Commands,
    gfx: Res<Gfx>,
    time: Res<Time>,
    added: Query<(Entity, &PowerUp), Added<PowerUp>>,
    mut all: Query<(&mut Sprite, &mut Transform), With<PowerUp>>,
) {
    for (e, p) in &added {
        commands.entity(e).insert(Sprite { image: gfx.powerup(p.0), custom_size: Some(Vec2::splat(16.0)), ..default() });
    }
    let t = time.elapsed_secs();
    for (mut s, mut tf) in &mut all {
        tf.translation.z = z::POWERUP;
        tf.scale = Vec3::splat(1.0 + 0.05 * (t * 6.0).sin());
        s.color = Color::WHITE.with_alpha(if ((t * 4.0) as u32).is_multiple_of(2) { 1.0 } else { 0.55 });
    }
}
