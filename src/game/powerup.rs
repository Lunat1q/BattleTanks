//! Power-ups: dropped by flashing enemies, picked up by players. Only one on the field at a time.

use super::*;
use crate::geom::{CELL, TILE, overlaps, rect_at};
use crate::rng::Rng;
use crate::terrain::{Terrain, Tile};

/// Random cell-aligned spot away from the base. Pure for testing.
pub fn random_spot(rng: &mut Rng) -> Vec2 {
    loop {
        let p = Vec2::new((2 + rng.below(23)) as f32 * CELL, (2 + rng.below(23)) as f32 * CELL);
        if !(p.y < 40.0 && (72.0..136.0).contains(&p.x)) {
            return p;
        }
    }
}

pub fn spawn_random(commands: &mut Commands, rng: &mut Rng, existing: &Query<Entity, With<PowerUp>>, sfx: &mut MessageWriter<Sfx>) {
    for e in existing {
        commands.entity(e).try_despawn();
    }
    commands.spawn((
        PowerUp(rng.pick(&rules::PowerKind::ALL)),
        Transform::from_translation(random_spot(rng).extend(0.0)),
        DespawnOnExit(Screen::Stage),
    ));
    sfx.write(Sfx::PowerAppear);
}

pub fn pickup(
    mut commands: Commands,
    mut session: ResMut<Session>,
    mut run: ResMut<StageRun>,
    mut terrain: ResMut<Terrain>,
    powerups: Query<(Entity, &PowerUp, &Transform)>,
    mut players: Query<(Entity, &Tank, &Transform, &mut Gun), (Without<Enemy>, Without<Spawning>)>,
    enemies: Query<(Entity, &Transform), With<Enemy>>,
    mut fx: MessageWriter<Fx>,
    mut sfx: MessageWriter<Sfx>,
) {
    for (pe, power, ptf) in &powerups {
        let prect = rect_at(ptf.translation.xy(), TILE);
        let Some((te, tank, _, mut gun)) = players.iter_mut().find(|(_, _, tf, _)| overlaps(prect, rect_at(tf.translation.xy(), TILE)))
        else {
            continue;
        };
        let Team::Player(p) = tank.team else { continue };
        commands.entity(pe).despawn();
        sfx.write(Sfx::PowerPick);
        fx.write(Fx { kind: FxKind::Points(rules::POWERUP_POINTS), pos: ptf.translation.xy() });
        let mut extra = session.add_score(p, rules::POWERUP_POINTS);
        match power.0 {
            rules::PowerKind::Helmet => {
                commands.entity(te).insert(Shield(rules::HELMET_SHIELD));
            }
            rules::PowerKind::Clock => run.freeze = rules::CLOCK_FREEZE,
            rules::PowerKind::Shovel => {
                run.shovel = rules::SHOVEL_TIME;
                terrain.set_base_ring(Tile::Steel);
            }
            rules::PowerKind::Star => {
                if let Some(slot) = session.slots[p].as_mut() {
                    slot.level = (slot.level + 1).min(3);
                    gun.stats = rules::player_gun(slot.level);
                }
            }
            rules::PowerKind::Grenade => {
                // No points for grenade kills, like the classic.
                for (e, tf) in &enemies {
                    commands.entity(e).try_despawn();
                    fx.write(Fx { kind: FxKind::TankBoom, pos: tf.translation.xy() });
                }
                sfx.write(Sfx::EnemyBoom);
            }
            rules::PowerKind::Tank => {
                if let Some(slot) = session.slots[p].as_mut() {
                    slot.lives += 1;
                    extra += 1;
                }
            }
        }
        if extra > 0 {
            sfx.write(Sfx::ExtraLife);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spots_avoid_the_base_and_stay_inside() {
        let mut rng = Rng::new(3);
        for _ in 0..500 {
            let p = random_spot(&mut rng);
            assert!((16.0..=192.0).contains(&p.x) && (16.0..=192.0).contains(&p.y), "{p}");
            assert!(!(p.y < 40.0 && (72.0..136.0).contains(&p.x)), "{p}");
        }
    }
}
