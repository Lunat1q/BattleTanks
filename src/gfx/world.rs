//! Sprites for logic entities: terrain sub-cells, tanks, bullets, base, power-ups.

use super::{Gfx, enemy_color, player_color, z};
use crate::game::*;
use crate::geom::{SUB, SUBS};
use crate::terrain::{Terrain, Tile};
use bevy::prelude::*;

/// Terrain sprite per sub-cell, rebuilt each stage.
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
    app.add_systems(OnEnter(Screen::Stage), spawn_floor).add_systems(
        Update,
        (sync_terrain, animate_water, attach_tanks, tank_looks, shield_rings, attach_bullets, base_look, powerup_looks)
            .run_if(in_state(Screen::Stage))
            .after(Sim),
    );
}

fn spawn_floor(mut commands: Commands, gfx: Res<Gfx>) {
    commands.insert_resource(TerrainSprites(vec![None; SUBS * SUBS]));
    let center = Vec3::new(104.0, 104.0, z::GROUND);
    commands.spawn((
        Sprite { color: Color::srgb(0.22, 0.25, 0.30), custom_size: Some(Vec2::splat(212.0)), ..default() },
        Transform::from_translation(center - Vec3::Z * 0.1),
        DespawnOnExit(Screen::Stage),
    ));
    commands.spawn((
        Sprite { image: gfx.ground.clone(), custom_size: Some(Vec2::splat(208.0)), ..default() },
        Transform::from_translation(center),
        DespawnOnExit(Screen::Stage),
    ));
}

fn sync_terrain(mut commands: Commands, gfx: Res<Gfx>, terrain: Option<ResMut<Terrain>>, mut sprites: ResMut<TerrainSprites>) {
    let Some(mut terrain) = terrain else { return };
    if terrain.dirty.is_empty() {
        return;
    }
    for (x, y) in std::mem::take(&mut terrain.dirty) {
        let i = y * SUBS + x;
        if let Some(old) = sprites.0[i].take() {
            commands.entity(old).try_despawn();
        }
        let (image, depth) = match terrain.get(x, y) {
            Tile::Empty => continue,
            Tile::Brick => (gfx.brick.clone(), z::WALL),
            Tile::Steel => (gfx.steel.clone(), z::WALL),
            Tile::Water => (gfx.water[0].clone(), z::FLOOR),
            Tile::Ice => (gfx.ice.clone(), z::FLOOR),
            Tile::Trees => (gfx.trees.clone(), z::TREES),
        };
        // Sample the matching quarter of the 16-unit tile so patterns line up across cells.
        let (qx, qy) = ((x % 4) as f32, (3 - y % 4) as f32);
        let rect = Rect::new(qx * SUB_PX, qy * SUB_PX, (qx + 1.0) * SUB_PX, (qy + 1.0) * SUB_PX);
        let pos = Vec3::new(x as f32 * SUB + SUB / 2.0, y as f32 * SUB + SUB / 2.0, depth);
        let mut e = commands.spawn((
            Sprite { image, rect: Some(rect), custom_size: Some(Vec2::splat(SUB)), ..default() },
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

fn attach_tanks(mut commands: Commands, gfx: Res<Gfx>, added: Query<Entity, Added<Tank>>, tanks: Query<&Transform>) {
    for e in &added {
        let Ok(tf) = tanks.get(e) else { continue };
        commands.entity(e).insert((Sprite { custom_size: Some(Vec2::splat(16.0)), ..default() }, Visibility::Hidden)).with_child((
            Sprite { image: gfx.dot.clone(), color: Color::srgba(0.0, 0.0, 0.0, 0.55), custom_size: Some(Vec2::splat(21.0)), ..default() },
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
    mut tanks: Query<(&Tank, Option<&Enemy>, &mut Sprite, &mut Transform, &mut Visibility, Has<Spawning>, Has<Paralyzed>, Has<HitFlash>)>,
) {
    let t = time.elapsed_secs();
    let blink = ((t * 8.0) as u32).is_multiple_of(2);
    for (tank, enemy, mut sprite, mut tf, mut vis, spawning, paralyzed, flash) in &mut tanks {
        vis.set_if_neq(if spawning { Visibility::Hidden } else { Visibility::Inherited });
        tf.translation.z = z::TANK;
        tf.rotation = Quat::from_rotation_z(tank.dir.angle());
        let frame = (tank.odometer / 1.5) as usize % 2;
        let (image, mut color) = match (tank.team, enemy) {
            (Team::Player(i), _) => {
                let level = session.slots[i].as_ref().map_or(0, |s| s.level) as usize;
                (gfx.players[level.min(3)][frame].clone(), player_color(i))
            }
            (Team::Enemy, Some(e)) => {
                let mut c = enemy_color(e.kind, e.hp);
                if e.bonus && blink {
                    c = Color::srgb(1.0, 0.32, 0.28);
                }
                if run.freeze > 0.0 {
                    c = c.mix(&Color::srgb(0.55, 0.8, 1.0), 0.5);
                }
                (gfx.enemies[e.kind.index()][frame].clone(), c)
            }
            (Team::Enemy, None) => continue,
        };
        if flash {
            color = Color::srgb(2.5, 2.5, 2.5);
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
