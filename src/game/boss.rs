//! District boss attacks (City Run), so the last fight is more than a big mini-boss:
//! - barrage: every `rules::boss_barrage_gap` the boss stops and its row and column light up for
//!   `rules::BOSS_CHARGE` (gfx draws the warning), then three shells fly each way along them;
//! - sentries: from 2/3 HP down each barrage also calls in sentry guns on open ground away from every tank (the
//!   spawn twinkle and a red frame warn first); a sentry fires all four ways at once. They fall with the boss.

use super::city::{CityRun, Reach};
use super::stage::spawn_enemy;
use super::*;
use crate::geom::{FIELD, overlaps, rect_at};
use crate::rng::Rng;
use crate::rules::{self, Rank};
use crate::terrain::Terrain;
use bevy::math::Rect;

/// Sideways offsets of the barrage's three shells per side; the lit band is `BAND` wide.
const LANES: [f32; 3] = [-10.0, 0.0, 10.0];
pub const BAND: f32 = 24.0;
/// A sentry lands at least this far from every tank.
const SENTRY_ROOM: f32 = 24.0;

/// Barrage shells for a boss at `pos` with side `size`: start and heading, three per side.
pub fn barrage(pos: Vec2, size: f32) -> Vec<(Vec2, Dir)> {
    Dir::ALL
        .iter()
        .flat_map(|&d| {
            let side = Vec2::new(-d.vec().y, d.vec().x);
            LANES.map(move |o| (pos + d.vec() * (size / 2.0 - 2.0) + side * o, d))
        })
        .collect()
}

/// Sentry spots in the view starting at `bottom`, in a fixed order (LAN: both machines pick the same one): cell
/// centres where `open` holds and no tank in `tanks` is within `SENTRY_ROOM`.
pub fn sentry_spots(bottom: f32, open: impl Fn(Vec2) -> bool, tanks: &[Rect]) -> Vec<Vec2> {
    let y0 = (bottom / 8.0).ceil() * 8.0 + 16.0;
    (0..((FIELD - 32.0) / 8.0) as usize)
        .flat_map(|row| (1..FIELD as usize / 8).map(move |col| Vec2::new(col as f32 * 8.0, y0 + row as f32 * 8.0)))
        .filter(|&p| {
            let room = rect_at(p, 16.0 + 2.0 * SENTRY_ROOM);
            open(p) && !tanks.iter().any(|&t| overlaps(room, t))
        })
        .collect()
}

pub fn act(
    mut commands: Commands,
    time: Res<Time>,
    run: Res<StageRun>,
    session: Res<Session>,
    view: Res<View>,
    city: Option<Res<CityRun>>,
    reach: Option<Res<Reach>>,
    terrain: Res<Terrain>,
    mut rng: ResMut<Rng>,
    mut tanks: Query<(Entity, &Transform, &mut Tank, Option<&mut Gun>, Option<&mut Enemy>, Has<Spawning>)>,
    mut fx: MessageWriter<Fx>,
    mut sfx: MessageWriter<Sfx>,
    ids: Ids,
) {
    let (Some(city), Some(reach)) = (city, reach) else { return };
    let dt = time.delta_secs();
    let boss_up = city.boss.is_some_and(|b| tanks.contains(b));
    let rects: Vec<Rect> = tanks.iter().map(|(_, tf, t, ..)| rect_at(tf.translation.xy(), t.size)).collect();
    let sentries = tanks.iter().filter(|(.., e, _)| e.as_ref().is_some_and(|e| e.rank == Rank::Sentry)).count();
    let mut shells: Vec<(Entity, Vec2, Dir, f32)> = Vec::new();
    let mut call = 0;
    // Spawn order: each sentry draws its next fire gap from the shared dice.
    for (e, tf, mut tank, gun, enemy, spawning) in tanks.iter_mut().sort_by_key::<Entity, _>(|e| order(&ids, *e)) {
        let (Some(mut gun), Some(mut ai)) = (gun, enemy) else { continue };
        let pos = tf.translation.xy();
        match ai.rank {
            Rank::Sentry if !boss_up => {
                commands.entity(e).try_despawn();
                fx.write(Fx { kind: FxKind::TankBoom, pos });
            }
            _ if spawning || run.freeze > 0.0 || run.phase != Phase::Play => {}
            Rank::Sentry => {
                ai.fire_in -= dt;
                if ai.fire_in <= 0.0 {
                    let (lo, hi) = ai.rank.fire_gap(session.stage);
                    ai.fire_in = rng.range(lo, hi);
                    shells.extend(Dir::ALL.map(|d| (e, pos + d.vec() * 6.0, d, gun.stats.bullet_speed)));
                    // A quarter turn per volley: it reads as a spinning gun.
                    tank.dir = Dir::ALL[(Dir::ALL.iter().position(|&d| d == tank.dir).unwrap_or(0) + 1) % 4];
                }
            }
            Rank::Boss => {
                let was = ai.skill;
                ai.skill -= dt;
                if was >= 0.0 && ai.skill < 0.0 {
                    sfx.write(Sfx::BossCharge);
                }
                if ai.skill < 0.0 {
                    // Charging: holds still, saves its shells for the barrage.
                    tank.want = None;
                    gun.trigger = false;
                }
                if ai.skill <= -rules::BOSS_CHARGE {
                    ai.skill = rules::boss_barrage_gap(session.stage);
                    shells.extend(barrage(pos, tank.size).into_iter().map(|(p, d)| (e, p, d, gun.stats.bullet_speed * 0.75)));
                    sfx.write(Sfx::Barrage);
                    if ai.hp * 3 <= ai.max_hp * 2 {
                        call = rules::sentry_cap(session.stage).saturating_sub(sentries);
                    }
                }
            }
            _ => {}
        }
    }
    for (owner, pos, dir, speed) in shells {
        commands.spawn((
            Bullet { owner, team: Team::Enemy, dir, speed, power: false, trees: false },
            Transform::from_translation(pos.extend(0.0)),
            DespawnOnExit(Screen::Stage),
        ));
        fx.write(Fx { kind: FxKind::Muzzle(dir), pos: pos + dir.vec() * 2.0 });
    }
    if call == 0 {
        return;
    }
    let mut taken = rects;
    for _ in 0..call {
        let spots = sentry_spots(view.bottom, |p| reach.ok(p) && !terrain.blocks_tank(rect_at(p, 16.0)), &taken);
        if spots.is_empty() {
            break;
        }
        let p = spots[rng.below(spots.len() as u32) as usize];
        spawn_enemy(&mut commands, EnemyKind::Basic, Rank::Sentry, p, false, session.stage, &mut rng);
        taken.push(rect_at(p, 16.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn barrage_fills_the_lit_band_each_way() {
        let shells = barrage(Vec2::new(100.0, 300.0), 32.0);
        assert_eq!(shells.len(), 12);
        for d in Dir::ALL {
            let lane: Vec<Vec2> = shells.iter().filter(|s| s.1 == d).map(|s| s.0).collect();
            assert_eq!(lane.len(), 3);
            for p in lane {
                let off = if d.vec().x == 0.0 { p.x - 100.0 } else { p.y - 300.0 };
                assert!(off.abs() + 2.0 <= BAND / 2.0, "{d:?} shell outside the warning band");
            }
        }
    }

    #[test]
    fn sentries_land_in_view_clear_of_tanks() {
        let player = rect_at(Vec2::new(104.0, 120.0), 16.0);
        let spots = sentry_spots(100.0, |_| true, &[player]);
        assert!(!spots.is_empty());
        for p in &spots {
            assert!(p.y - 8.0 >= 100.0 && p.y + 8.0 <= 100.0 + FIELD && p.x >= 8.0 && p.x <= FIELD - 8.0, "{p}");
            assert!(p.distance(Vec2::new(104.0, 120.0)) >= 16.0 + SENTRY_ROOM, "{p} too close");
        }
        assert!(sentry_spots(100.0, |_| false, &[]).is_empty(), "never on blocked ground");
    }
}
