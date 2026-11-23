//! Power-ups: dropped by flashing enemies when they die, picked up by players. A new drop leaves earlier ones be.

use super::*;
use crate::geom::{CELL, CELLS, TILE, overlaps, rect_at};
use crate::levels;
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

/// A classic drop spot a player can drive onto: no brick, steel or water under it, and reachable from player 1's
/// spawn without crossing steel or water. Regression (owner): drops landed inside steel and in water.
pub fn classic_spot(rng: &mut Rng, ter: &Terrain) -> Vec2 {
    let reach = levels::steps(ter, (8, 0), levels::hard_wall);
    let ok = |p: Vec2| {
        let (x, y) = ((p.x / CELL) as usize - 1, (p.y / CELL) as usize - 1);
        !ter.blocks_tank(rect_at(p, TILE)) && reach[y * CELLS + x].is_some()
    };
    // ponytail: 100 tries, then player 1's spawn (always open); a field that is nearly all wall would want a scan.
    (0..100).map(|_| random_spot(rng)).find(|&p| ok(p)).unwrap_or(Vec2::new(72.0, 8.0))
}

/// A VERSUS drop with no armies: open ground in the middle band (cell rows 10-15), reachable from player 1's spawn.
pub fn middle_spot(rng: &mut Rng, ter: &Terrain) -> Vec2 {
    let reach = levels::steps(ter, (8, 0), levels::hard_wall);
    let ok = |p: Vec2| {
        let (x, y) = ((p.x / CELL) as usize - 1, (p.y / CELL) as usize - 1);
        !ter.blocks_tank(rect_at(p, TILE)) && reach[y * CELLS + x].is_some()
    };
    // ponytail: 100 tries, then the field's centre (usually open); a walled middle would want a scan.
    (0..100)
        .map(|_| Vec2::new((2 + rng.below(23)) as f32 * CELL, (11 + rng.below(5)) as f32 * CELL))
        .find(|&p| ok(p))
        .unwrap_or(Vec2::splat(104.0))
}

/// City Run has no base, so no shovel.
const CITY_POWERS: [rules::PowerKind; 5] =
    [rules::PowerKind::Helmet, rules::PowerKind::Clock, rules::PowerKind::Star, rules::PowerKind::Grenade, rules::PowerKind::Tank];

/// Drops a power-up in the view (`bottom` up) where a tank can drive. Earlier drops stay (owner: a second bonus kill
/// took away the first drop).
pub fn spawn_random(commands: &mut Commands, rng: &mut Rng, sfx: &mut MessageWriter<Sfx>, bottom: f32, ter: &Terrain, city: bool) {
    let lift = Vec2::new(0.0, (bottom / CELL).ceil() * CELL);
    let (kind, pos) = match city {
        false => (rng.pick(&rules::PowerKind::ALL), classic_spot(rng, ter)),
        true => {
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
    enemies: Query<(Entity, &Transform, &Enemy, &Tank)>,
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
        let versus = session.mode == Mode::Versus;
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
            // VERSUS: the other side (player and NPCs) stands still.
            rules::PowerKind::Clock if versus => run.frozen[1 - p] = rules::VERSUS_FREEZE,
            rules::PowerKind::Clock => run.freeze = rules::CLOCK_FREEZE,
            rules::PowerKind::Shovel => {
                // VERSUS: your own base; classic has only base 0.
                let side = if versus { p } else { 0 };
                run.shovel[side] = rules::SHOVEL_TIME;
                terrain.set_ring(side, Tile::Steel);
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
                // No points for grenade kills, like the classic. Turrets and bosses shrug it off; the boss's sentries
                // don't. VERSUS: only the other side's NPCs.
                let mut count = 0;
                let hit = |en: &Enemy, t: &Tank| {
                    matches!(en.rank, rules::Rank::Normal | rules::Rank::Sentry) && (!versus || t.team == Team::Npc(1 - p))
                };
                for (e, tf, _, _) in enemies.iter().filter(|(_, _, en, t)| hit(en, t)) {
                    commands.entity(e).try_despawn();
                    fx.write(Fx { kind: FxKind::TankBoom, pos: tf.translation.xy() });
                    count += 1;
                }
                feats.write(Feat::Grenade { slot: p, count });
                sfx.write(Sfx::EnemyBoom);
            }
            // VERSUS has no spare tanks: the next respawn wait is one step shorter.
            rules::PowerKind::Tank if versus => {
                if let Some(slot) = session.slots[p].as_mut() {
                    slot.deaths = slot.deaths.saturating_sub(1);
                }
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

    #[test]
    fn classic_drops_land_where_a_player_can_drive() {
        let reach_of = |ter: &Terrain| levels::steps(ter, (8, 0), levels::hard_wall);
        let fine = |ter: &Terrain, reach: &[Option<u32>], p: Vec2| {
            !ter.blocks_tank(rect_at(p, TILE)) && reach[(p.y / CELL) as usize * CELLS - CELLS + (p.x / CELL) as usize - 1].is_some()
        };
        let mut rng = Rng::new(11);
        let mut old_misses = 0;
        for n in 1..=levels::stage_count() {
            let ter = levels::load(n).terrain;
            let reach = reach_of(&ter);
            for _ in 0..300 {
                let p = classic_spot(&mut rng, &ter);
                assert!(fine(&ter, &reach, p), "stage {n}: drop at {p} is walled in or blocked");
                old_misses += usize::from(!fine(&ter, &reach, random_spot(&mut rng)));
            }
        }
        assert!(old_misses > 0, "the plain picker should hit steel/water somewhere, or this test proves nothing");
    }
}
