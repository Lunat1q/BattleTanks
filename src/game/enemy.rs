//! Enemy AI: wander with a pull towards the base or a player, turn when blocked, fire at random.
//! Deliberately simple and a bit dumb, like the classic.

use super::stage::BASE_POS;
use super::*;
use crate::rng::Rng;

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

pub fn think(
    time: Res<Time>,
    run: Res<StageRun>,
    mut rng: ResMut<Rng>,
    players: Query<(&Transform, &Tank), (Without<Enemy>, Without<Spawning>)>,
    mut enemies: Query<(&Transform, &mut Tank, &mut Gun, &mut Enemy), Without<Spawning>>,
) {
    let dt = time.delta_secs();
    let player_pos: Vec<Vec2> =
        players.iter().filter(|(_, t)| matches!(t.team, Team::Player(_))).map(|(tf, _)| tf.translation.xy()).collect();
    for (tf, mut tank, mut gun, mut ai) in &mut enemies {
        if run.freeze > 0.0 {
            tank.want = None;
            gun.trigger = false;
            continue;
        }
        let pos = tf.translation.xy();
        ai.turn_in -= dt;
        if tank.blocked || ai.turn_in <= 0.0 || tank.want.is_none() {
            // Mostly the base; sometimes the nearest player.
            let target = if rng.chance(0.35) && !player_pos.is_empty() {
                *player_pos.iter().min_by(|a, b| a.distance(pos).total_cmp(&b.distance(pos))).unwrap()
            } else {
                BASE_POS
            };
            let roll = rng.f32();
            tank.want = Some(choose_dir(pos, target, tank.dir, tank.blocked, roll, rng.f32()));
            ai.turn_in = if tank.blocked { rng.range(0.2, 0.8) } else { rng.range(0.8, 3.0) };
            if tank.blocked && rng.chance(0.5) {
                ai.fire_in = 0.0; // shoot what is in the way
            }
        }
        ai.fire_in -= dt;
        gun.trigger = ai.fire_in <= 0.0;
        if gun.trigger {
            ai.fire_in = rng.range(0.4, 1.6);
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
    fn blocked_never_repeats_the_blocked_heading() {
        for i in 0..100 {
            let r = i as f32 / 100.0;
            for cur in Dir::ALL {
                let d = choose_dir(Vec2::new(100.0, 200.0), BASE_POS, cur, true, r, r);
                assert_ne!(d, cur, "roll {r} cur {cur:?}");
            }
        }
    }
}
