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

/// City Run has no base, so no shovel.
const CITY_POWERS: [rules::PowerKind; 5] =
    [rules::PowerKind::Helmet, rules::PowerKind::Clock, rules::PowerKind::Star, rules::PowerKind::Grenade, rules::PowerKind::Tank];

/// Drops a power-up in the view, replacing the last drop (stashes stay) (`bottom` up). `city`: City Run terrain, so the drop lands where a tank can drive.
pub fn spawn_random(
    commands: &mut Commands,
    rng: &mut Rng,
    existing: &Query<Entity, (With<PowerUp>, Without<Stash>)>,
    sfx: &mut MessageWriter<Sfx>,
    bottom: f32,
    city: Option<&Terrain>,
) {
    for e in existing {
        commands.entity(e).try_despawn();
    }
    let lift = Vec2::new(0.0, (bottom / CELL).ceil() * CELL);
    let (kind, pos) = match city {
        None => (rng.pick(&rules::PowerKind::ALL), random_spot(rng) + lift),
        Some(ter) => {
            // ponytail: 40 random tries, then the view centre; fine for maps that aren't mostly wall.
            let open = (0..40).map(|_| random_spot(rng) + lift).find(|&p| !ter.blocks_tank(rect_at(p, TILE)));
            (rng.pick(&CITY_POWERS), open.unwrap_or(lift + Vec2::splat(104.0)))
        }
    };
    commands.spawn((PowerUp(kind), Transform::from_translation(pos.extend(0.0)), DespawnOnExit(Screen::Stage)));
    sfx.write(Sfx::PowerAppear);
}

pub fn pickup(
    mut commands: Commands,
    mut session: ResMut<Session>,
    mut run: ResMut<StageRun>,
    mut terrain: ResMut<Terrain>,
    powerups: Query<(Entity, &PowerUp, &Transform)>,
    mut players: Query<(Entity, &Tank, &Transform, &mut Gun), (Without<Enemy>, Without<Spawning>)>,
    enemies: Query<(Entity, &Transform, &Enemy)>,
    mut fx: MessageWriter<Fx>,
    mut sfx: MessageWriter<Sfx>,
    mut rumble: MessageWriter<Rumble>,
    ids: Ids,
    mut feats: MessageWriter<Feat>,
) {
    for (pe, power, ptf) in powerups.iter().sort_by_key::<Entity, _>(|e| order(&ids, *e)) {
        let prect = rect_at(ptf.translation.xy(), TILE);
        // Spawn order: if both players touch it, the same one gets it on every machine.
        let Some((te, tank, _, mut gun)) = players
            .iter_mut()
            .sort_by_key::<Entity, _>(|e| order(&ids, *e))
            .find(|(_, _, tf, _)| overlaps(prect, rect_at(tf.translation.xy(), TILE)))
        else {
            continue;
        };
        let Team::Player(p) = tank.team else { continue };
        commands.entity(pe).despawn();
        sfx.write(Sfx::PowerPick);
        rumble.write(Rumble::tick(p));
        fx.write(Fx { kind: FxKind::Points(rules::POWERUP_POINTS), pos: ptf.translation.xy() });
        let mut extra = session.add_score(p, rules::POWERUP_POINTS);
        if let Some(slot) = session.slots[p].as_mut() {
            slot.pickups += 1;
        }
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
                    slot.level = (slot.level + 1).min(rules::MAX_LEVEL);
                    gun.stats = rules::player_gun(slot.level);
                    // The new look flashes in.
                    commands.entity(te).insert(HitFlash(0.25));
                    feats.write(Feat::Star { slot: p, level: slot.level });
                }
            }
            rules::PowerKind::Grenade => {
                // No points for grenade kills, like the classic. Turrets and bosses shrug it off.
                let mut count = 0;
                for (e, tf, _) in enemies.iter().filter(|(_, _, en)| en.rank == rules::Rank::Normal) {
                    commands.entity(e).try_despawn();
                    fx.write(Fx { kind: FxKind::TankBoom, pos: tf.translation.xy() });
                    count += 1;
                }
                feats.write(Feat::Grenade { slot: p, count });
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
