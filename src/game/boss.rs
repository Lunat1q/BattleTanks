//! District boss attacks (City Run), so the last fight is more than a big mini-boss:
//! - barrage: every `rules::boss_barrage_gap` the boss stops and its row and column light up for
//!   `rules::BOSS_CHARGE` (gfx draws the warning), then three shells fly each way along them;
//! - sentries: from 2/3 HP down each barrage also calls in sentry guns on open ground away from every tank (the
//!   spawn twinkle and a red frame warn first); a sentry fires all four ways at once. They fall with the boss.
//! - later districts add tricks (`rules::boss_kit`): an echo barrage, a brick ring at 1/2 HP, a rage at 1/3 HP,
//!   and lines lit through the nearest player that shells sweep down from the view's edges.

use super::city::{CityRun, Reach};
use super::stage::spawn_enemy;
use super::*;
use crate::geom::{FIELD, overlaps, rect_at};
use crate::rng::Rng;
use crate::rules::{self, Rank};
use crate::terrain::{Terrain, Tile};
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

/// Shells down the row and column marked through a player at `m`, coming in from the view's left and top edges.
pub fn marked(m: Vec2, bottom: f32) -> Vec<(Vec2, Dir)> {
    LANES.iter().flat_map(|&o| [(Vec2::new(6.0, m.y + o), Dir::Right), (Vec2::new(m.x + o, bottom + FIELD - 6.0), Dir::Down)]).collect()
}

/// Sub-cells of the brick ring a fortifying boss at `pos` raises: 8 units thick, one cell out from a 32-wide boss,
/// so it can still shuffle inside and shoot its way out (as can players).
pub fn ring(pos: Vec2) -> Vec<(usize, usize)> {
    let lo = |v: f32| ((v - 32.0) / 4.0).floor().max(0.0) as usize;
    (lo(pos.y)..lo(pos.y) + 16)
        .flat_map(|y| (lo(pos.x)..lo(pos.x) + 16).map(move |x| (x, y)))
        .filter(|&(x, y)| {
            let d = (x as f32 * 4.0 + 2.0 - pos.x).abs().max((y as f32 * 4.0 + 2.0 - pos.y).abs());
            (24.0..32.0).contains(&d)
        })
        .collect()
}

/// How lit the boss's bands are, 0..=1: rising through the charge (`skill` below 0), full while an echo is due.
pub fn glow(skill: f32, gap: f32, echo: bool) -> Option<f32> {
    if skill < 0.0 {
        Some((-skill / rules::BOSS_CHARGE).min(1.0))
    } else if echo && skill > gap {
        Some(1.0)
    } else {
        None
    }
}

pub fn act(
    mut commands: Commands,
    time: Res<Time>,
    run: Res<StageRun>,
    session: Res<Session>,
    view: Res<View>,
    city: Option<Res<CityRun>>,
    reach: Option<Res<Reach>>,
    mut terrain: ResMut<Terrain>,
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
    // Spawn order: a tie for the nearest player goes the same way on every machine.
    let players: Vec<Vec2> = tanks
        .iter()
        .sort_by_key::<Entity, _>(|e| order(&ids, *e))
        .filter(|(_, _, t, _, e, spawning)| e.is_none() && !spawning && matches!(t.team, Team::Player(_)))
        .map(|(_, tf, ..)| tf.translation.xy())
        .collect();
    let kit = rules::boss_kit(session.stage);
    let mut shells: Vec<(Entity, Vec2, Dir, f32)> = Vec::new();
    let mut call = 0;
    let mut fortify = None;
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
                // Each HP phase acts once.
                while ai.phase < rules::boss_phase(ai.hp, ai.max_hp) {
                    ai.phase += 1;
                    match ai.phase {
                        2 if kit.fortify => fortify = Some(pos),
                        3 if kit.rage => {
                            tank.speed *= rules::RAGE_SPEED;
                            ai.skill = ai.skill.min(0.0); // charges at once: the rage announces itself
                        }
                        _ => {}
                    }
                }
                let gap = rules::boss_barrage_gap(session.stage, kit.rage && ai.phase >= 3);
                let speed = gun.stats.bullet_speed * 0.75;
                let was = ai.skill;
                ai.skill -= dt;
                if was >= 0.0 && ai.skill < 0.0 {
                    sfx.write(Sfx::BossCharge);
                    let nearest = players.iter().copied().min_by(|a, b| a.distance(pos).total_cmp(&b.distance(pos)));
                    ai.mark = nearest.filter(|_| kit.mark);
                }
                if kit.echo && was > gap && ai.skill <= gap {
                    shells.extend(barrage(pos, tank.size).into_iter().map(|(p, d)| (e, p, d, speed)));
                    sfx.write(Sfx::Barrage);
                }
                if glow(ai.skill, gap, kit.echo).is_some() {
                    // Charging or with an echo due: holds still, saves its shells for the barrage.
                    tank.want = None;
                    gun.trigger = false;
                }
                if ai.skill <= -rules::BOSS_CHARGE {
                    ai.skill = gap + if kit.echo { rules::BOSS_ECHO } else { 0.0 };
                    shells.extend(barrage(pos, tank.size).into_iter().map(|(p, d)| (e, p, d, speed)));
                    if let Some(m) = ai.mark.take() {
                        shells.extend(marked(m, view.bottom).into_iter().map(|(p, d)| (e, p, d, speed)));
                    }
                    sfx.write(Sfx::Barrage);
                    if ai.phase >= 1 {
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
    if let Some(pos) = fortify {
        let rows = terrain.rows();
        for (x, y) in ring(pos) {
            let cell = Rect::new(x as f32 * 4.0, y as f32 * 4.0, x as f32 * 4.0 + 4.0, y as f32 * 4.0 + 4.0);
            let free = x < crate::geom::SUBS && y < rows && terrain.get(x, y) == Tile::Empty;
            if free && !rects.iter().any(|&t| overlaps(cell, t)) {
                terrain.set(x, y, Tile::Brick);
            }
        }
        fx.write(Fx { kind: FxKind::Bricks, pos });
        sfx.write(Sfx::Brick);
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
    fn marked_lines_fill_the_band_through_the_player() {
        let m = Vec2::new(60.0, 340.0);
        let shells = marked(m, 300.0);
        assert_eq!(shells.len(), 6);
        for (p, d) in shells {
            let off = if d == Dir::Right { p.y - m.y } else { p.x - m.x };
            assert!(off.abs() + 2.0 <= BAND / 2.0);
            assert!(p.x > 0.0 && p.y < 300.0 + FIELD, "starts inside the view, or the edge pops it");
        }
    }

    #[test]
    fn the_ring_leaves_the_boss_room_and_is_one_cell_thick() {
        let pos = Vec2::new(104.0, 500.0);
        let cells = ring(pos);
        assert_eq!(cells.len(), 16 * 16 - 12 * 12, "a full square band two sub-cells wide");
        for (x, y) in cells {
            let c = rect_at(Vec2::new(x as f32 * 4.0 + 2.0, y as f32 * 4.0 + 2.0), 4.0);
            assert!(!overlaps(c, rect_at(pos, 32.0 + 16.0)), "a cell of room left round the boss");
            assert!(overlaps(c, rect_at(pos, 64.0)));
        }
    }

    #[test]
    fn bands_glow_through_the_charge_and_until_the_echo() {
        assert_eq!(glow(1.0, 5.0, true), None);
        assert_eq!(glow(5.2, 5.0, true), Some(1.0), "echo due");
        assert_eq!(glow(5.2, 5.0, false), None);
        assert!(glow(-0.6, 5.0, false).is_some_and(|g| g > 0.0 && g < 1.0));
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
