//! Enemy AI: wander with a pull towards the base or a player, turn when blocked, fire at random.
//! Deliberately simple and a bit dumb, like the classic.

use super::stage::BASE_POS;
use super::*;
use crate::geom::FIELD;
use crate::rng::Rng;

/// Turrets sit in a full-width wall: a sideways shot only hits the wall, so they aim up or down only. Swinging
/// sideways at a player beside them let the other one shoot them from below unanswered.
pub fn turret_dir(pos: Vec2, target: Vec2) -> Dir {
    if target.y > pos.y { Dir::Up } else { Dir::Down }
}

/// Would a shell from `pos` heading `dir` fly into side `side`'s base or its brick ring? A VERSUS NPC holds its
/// fire then (owner: they shot holes in their own ring).
pub fn into_own_fort(pos: Vec2, dir: Dir, side: usize) -> bool {
    let fort = Rect::new(88.0, 0.0, 120.0, 24.0);
    let fort = if side == 0 { fort } else { Rect::new(FIELD - fort.max.x, FIELD - fort.max.y, FIELD - fort.min.x, FIELD - fort.min.y) };
    // The shell's lane: 4 wide, from the tank to past the field's edge.
    let lane = Rect::from_corners(pos, pos + dir.vec() * 2.0 * FIELD).inflate(2.0);
    crate::geom::overlaps(lane, fort)
}

/// Pick a heading. Pure so it can be tested; `roll` in [0, 1).
pub fn choose_dir(pos: Vec2, target: Vec2, current: Dir, blocked: bool, roll: f32, roll2: f32) -> Dir {
    if roll < 0.45 {
        // Head for the target along the longer axis (or the other one if that is what's blocked).
        let d = target - pos;
        let horizontal = if d.x > 0.0 { Dir::Right } else { Dir::Left };
        let vertical = if d.y > 0.0 { Dir::Up } else { Dir::Down };
        let (first, second) = if d.x.abs() > d.y.abs() { (horizontal, vertical) } else { (vertical, horizontal) };
        return if blocked && first == current { second } else { first };
    }
    // Weighted wander, biased downwards (towards the base side).
    let pick = match roll2 {
        r if r < 0.4 => Dir::Down,
        r if r < 0.6 => Dir::Left,
        r if r < 0.8 => Dir::Right,
        _ => Dir::Up,
    };
    if blocked && pick == current {
        Dir::ALL[(Dir::ALL.iter().position(|&d| d == pick).unwrap() + 1 + (roll2 * 30.0) as usize % 3) % 4]
    } else {
        pick
    }
}

/// `choose_dir` for a tank whose way forward is up (a VERSUS NPC of player 1): the same pick in the field turned
/// 180 degrees, so its wander drifts up towards the other side.
pub fn choose_dir_up(pos: Vec2, target: Vec2, current: Dir, blocked: bool, roll: f32, roll2: f32) -> Dir {
    let f = Vec2::splat(FIELD);
    turned(choose_dir(f - pos, f - target, turned(current), blocked, roll, roll2))
}

fn turned(d: Dir) -> Dir {
    match d {
        Dir::Up => Dir::Down,
        Dir::Down => Dir::Up,
        Dir::Left => Dir::Right,
        Dir::Right => Dir::Left,
    }
}

pub fn think(
    time: Res<Time>,
    run: Res<StageRun>,
    session: Res<Session>,
    base: Query<(), With<Base>>,
    mut rng: ResMut<Rng>,
    players: Query<(&Transform, &Tank), (Without<Enemy>, Without<Spawning>)>,
    mut enemies: Query<(&Transform, &mut Tank, &mut Gun, &mut Enemy), Without<Spawning>>,
    ids: Ids,
) {
    let dt = time.delta_secs();
    // Spawn order: on a tie for nearest (an enemy halfway between the two players) the first one wins.
    let player_pos: Vec<Vec2> = players
        .iter()
        .sort_by_key::<Entity, _>(|e| order(&ids, *e))
        .filter(|(_, t)| matches!(t.team, Team::Player(_)))
        .map(|(tf, _)| tf.translation.xy())
        .collect();
    // VERSUS NPCs hunt the other side: its player and its NPCs (spawn order again, for ties).
    let npcs: Vec<(Team, Vec2)> =
        enemies.iter().sort_by_key::<Entity, _>(|e| order(&ids, *e)).map(|(tf, t, _, _)| (t.team, tf.translation.xy())).collect();
    let tanks: Vec<(Team, Vec2)> =
        players.iter().sort_by_key::<Entity, _>(|e| order(&ids, *e)).map(|(tf, t)| (t.team, tf.translation.xy())).chain(npcs).collect();
    // Spawn order: each enemy draws from the shared dice, so the order decides who gets which roll.
    for (tf, mut tank, mut gun, mut ai) in enemies.iter_mut().sort_by_key::<Entity, _>(|e| order(&ids, *e)) {
        let frozen = match tank.team {
            Team::Npc(side) => run.frozen[side] > 0.0,
            _ => run.freeze > 0.0,
        };
        if frozen {
            tank.want = None;
            gun.trigger = false;
            continue;
        }
        let pos = tf.translation.xy();
        let nearest = player_pos.iter().copied().min_by(|a, b| a.distance(pos).total_cmp(&b.distance(pos)));
        if ai.rank == Rank::Sentry {
            continue; // fires on its own clock (`boss`)
        }
        let (lo, hi) = ai.rank.fire_gap(session.stage);
        if ai.rank == Rank::Turret {
            // Bolted to the wall: swing the gun towards the nearest player and keep shooting.
            if let Some(p) = nearest {
                tank.dir = turret_dir(pos, p);
            }
            ai.fire_in -= dt;
            gun.trigger = nearest.is_some() && ai.fire_in <= 0.0;
            if gun.trigger {
                ai.fire_in = rng.range(lo, hi);
            }
            continue;
        }
        ai.turn_in -= dt;
        if tank.blocked || ai.turn_in <= 0.0 || tank.want.is_none() {
            let (target, up) = match tank.team {
                // VERSUS NPC: 60 % the other base, 40 % the nearest hostile tank.
                Team::Npc(side) => {
                    let hostile = tanks.iter().filter(|(t, _)| t.side() != Some(side)).map(|(_, p)| *p);
                    let near = hostile.min_by(|a, b| a.distance(pos).total_cmp(&b.distance(pos)));
                    let base = super::versus::BASES[1 - side];
                    (near.filter(|_| rng.chance(0.4)).unwrap_or(base), side == 0)
                }
                // Mostly the base; sometimes the nearest player. No base (City Run): always the players.
                _ => match nearest {
                    Some(p) if base.is_empty() || rng.chance(0.35) => (p, false),
                    _ => (BASE_POS, false),
                },
            };
            let roll = rng.f32();
            let pick = if up { choose_dir_up } else { choose_dir };
            tank.want = Some(pick(pos, target, tank.dir, tank.blocked, roll, rng.f32()));
            let (t0, t1) = if tank.blocked { (0.2, 0.8) } else { ai.kind.turn_gap() };
            ai.turn_in = rng.range(t0, t1);
            if tank.blocked && rng.chance(0.5) {
                ai.fire_in = 0.0; // shoot what is in the way
            }
        }
        ai.fire_in -= dt;
        gun.trigger = ai.fire_in <= 0.0;
        if gun.trigger {
            ai.fire_in = rng.range(lo, hi);
            if let Team::Npc(side) = tank.team {
                gun.trigger = !into_own_fort(pos, tank.dir, side);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hunting_goes_along_longer_axis() {
        let d = choose_dir(Vec2::new(100.0, 200.0), BASE_POS, Dir::Down, false, 0.1, 0.0);
        assert_eq!(d, Dir::Down);
        let d = choose_dir(Vec2::new(10.0, 20.0), BASE_POS, Dir::Down, false, 0.1, 0.0);
        assert_eq!(d, Dir::Right);
    }

    #[test]
    fn turret_never_aims_into_its_wall() {
        let t = Vec2::new(100.0, 300.0);
        assert_eq!(turret_dir(t, Vec2::new(80.0, 296.0)), Dir::Down, "player beside it, a bit lower");
        assert_eq!(turret_dir(t, Vec2::new(180.0, 300.0)), Dir::Down);
        assert_eq!(turret_dir(t, Vec2::new(100.0, 340.0)), Dir::Up);
    }

    #[test]
    fn blocked_never_repeats_the_blocked_heading() {
        for i in 0..100 {
            let r = i as f32 / 100.0;
            for cur in Dir::ALL {
                let d = choose_dir(Vec2::new(100.0, 200.0), BASE_POS, cur, true, r, r);
                assert_ne!(d, cur, "roll {r} cur {cur:?}");
            }
        }
    }

    #[test]
    fn npcs_never_fire_into_their_own_base_or_ring() {
        // Player 1's base is bottom centre, its ring x 88..120, y 0..24.
        assert!(into_own_fort(Vec2::new(104.0, 100.0), Dir::Down, 0), "straight down onto the base");
        assert!(into_own_fort(Vec2::new(90.0, 100.0), Dir::Down, 0), "onto the ring's edge");
        assert!(into_own_fort(Vec2::new(8.0, 16.0), Dir::Right, 0), "across the back row into the ring");
        assert!(!into_own_fort(Vec2::new(8.0, 30.0), Dir::Right, 0), "just above the ring");
        assert!(!into_own_fort(Vec2::new(104.0, 100.0), Dir::Up, 0), "away from it");
        assert!(!into_own_fort(Vec2::new(104.0, 100.0), Dir::Down, 1), "the other base is fair game");
        assert!(into_own_fort(Vec2::new(104.0, 100.0), Dir::Up, 1));
        assert!(into_own_fort(Vec2::new(200.0, 192.0), Dir::Left, 1), "player 2's back row");
    }

    #[test]
    fn player_one_npcs_wander_up_towards_the_other_side() {
        // Wander roll 0.1 is "towards the base side": down for enemies and player 2's NPCs, up for player 1's.
        assert_eq!(choose_dir(Vec2::splat(100.0), Vec2::ZERO, Dir::Left, false, 0.9, 0.1), Dir::Down);
        assert_eq!(choose_dir_up(Vec2::splat(100.0), Vec2::ZERO, Dir::Left, false, 0.9, 0.1), Dir::Up);
        // Heading for a target still heads for it.
        assert_eq!(choose_dir_up(Vec2::new(104.0, 20.0), Vec2::new(104.0, 200.0), Dir::Left, false, 0.1, 0.5), Dir::Up);
    }
}
