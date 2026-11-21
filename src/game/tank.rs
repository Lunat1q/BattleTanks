//! Player control, tank movement with collisions, and firing.

use super::stage::BASE_POS;
use super::*;
use crate::geom::{TILE, overlaps, rect_at, snap};
use crate::terrain::{Terrain, Tile};
use bevy::math::Rect;

const AUTOFIRE_DELAY: f32 = 0.25;

pub fn player_control(
    session: Res<Session>,
    devices: Res<DeviceIntents>,
    run: Res<StageRun>,
    mut tanks: Query<(&mut Tank, &mut Gun, Has<Paralyzed>, Has<Spawning>)>,
) {
    for (mut tank, mut gun, paralyzed, spawning) in &mut tanks {
        let Team::Player(i) = tank.team else { continue };
        let intent = if run.phase == Phase::Lost || paralyzed || spawning { Intent::default() } else { session.intent(i, &devices) };
        tank.want = intent.dir;
        gun.trigger = intent.fire_pressed || (intent.fire && gun.cooldown <= 0.0);
    }
}

/// Move from `pos` up to `dist` along `dir`, stopping flush against whatever `blocked` reports.
pub fn advance(pos: Vec2, dir: Dir, dist: f32, blocked: impl Fn(Vec2) -> bool) -> Vec2 {
    let at = |d: f32| pos + dir.vec() * d;
    if !blocked(at(dist)) {
        return at(dist);
    }
    let (mut lo, mut hi) = (0.0, dist);
    for _ in 0..10 {
        let mid = (lo + hi) * 0.5;
        if blocked(at(mid)) { hi = mid } else { lo = mid }
    }
    at(lo)
}

pub fn drive(
    time: Res<Time>,
    session: Res<Session>,
    terrain: Res<Terrain>,
    view: Res<View>,
    base: Query<(), With<Base>>,
    mut tanks: Query<(Entity, &mut Tank, &mut Transform, Has<Spawning>)>,
    ids: Ids,
) {
    let dt = time.delta_secs().min(0.05);
    let mut rects: Vec<(Entity, Rect)> = tanks.iter().map(|(e, t, tf, _)| (e, rect_at(tf.translation.xy(), t.size))).collect();
    let base = (!base.is_empty()).then(|| rect_at(BASE_POS, TILE));

    // Spawn order: a tank that moves first takes the spot the next one wanted.
    for (e, mut tank, mut tf, spawning) in tanks.iter_mut().sort_by_key::<Entity, _>(|e| order(&ids, *e)) {
        if spawning {
            continue;
        }
        let mut pos = tf.translation.xy();
        let size = tank.size;
        let swimmer = matches!(tank.team, Team::Player(p) if session.slots[p].as_ref().is_some_and(|s| s.level >= rules::SWIM_LEVEL));
        let others: Vec<Rect> = rects
            .iter()
            .filter(|(o, r)| *o != e && !overlaps(*r, rect_at(pos, size))) // already overlapping: let them separate
            .map(|(_, r)| *r)
            .chain(base)
            .collect();
        let blocked = |p: Vec2| {
            let r = rect_at(p, size);
            terrain.blocks(r, swimmer) || r.min.y < view.bottom || r.max.y > view.top() || others.iter().any(|o| overlaps(*o, r))
        };

        let on_ice = terrain.at(pos.x, pos.y) == Tile::Ice;
        let go = match tank.want {
            Some(want) => {
                if want.is_vertical() != tank.dir.is_vertical() {
                    // Classic feel: turning snaps the old axis to the 8-unit grid so tanks fit gaps.
                    let snapped = if tank.dir.is_vertical() { Vec2::new(pos.x, snap(pos.y)) } else { Vec2::new(snap(pos.x), pos.y) };
                    if !blocked(snapped) {
                        pos = snapped;
                    }
                }
                tank.dir = want;
                tank.slide = if on_ice { rules::SLIDE_TIME } else { 0.0 };
                true
            }
            None if tank.slide > 0.0 => {
                tank.slide -= dt;
                true
            }
            None => false,
        };
        tank.moving = go;
        let dist = if go { tank.speed * dt } else { 0.0 };
        let new = advance(pos, tank.dir, dist, blocked);
        tank.blocked = go && new.distance(pos) < dist * 0.5;
        tank.odometer += new.distance(pos);
        tf.translation.x = new.x;
        tf.translation.y = new.y;
        if let Some(r) = rects.iter_mut().find(|(o, _)| *o == e) {
            r.1 = rect_at(new, size);
        }
    }
}

pub fn fire(
    mut commands: Commands,
    mut tanks: Query<(Entity, &Tank, &mut Gun, &Transform), Without<Spawning>>,
    bullets: Query<&Bullet>,
    mut sfx: MessageWriter<Sfx>,
    mut fx: MessageWriter<Fx>,
    ids: Ids,
) {
    // Spawn order, so bullets are numbered alike on every machine.
    for (e, tank, mut gun, tf) in tanks.iter_mut().sort_by_key::<Entity, _>(|e| order(&ids, *e)) {
        if !std::mem::take(&mut gun.trigger) {
            continue;
        }
        if bullets.iter().filter(|b| b.owner == e).count() >= gun.stats.max_bullets {
            continue;
        }
        let pos = tf.translation.xy() + tank.dir.vec() * (tank.size / 2.0 - 2.0);
        commands.spawn((
            Bullet {
                owner: e,
                team: tank.team,
                dir: tank.dir,
                speed: gun.stats.bullet_speed,
                power: gun.stats.power,
                trees: gun.stats.power,
            },
            Transform::from_translation(pos.extend(0.0)),
            DespawnOnExit(Screen::Stage),
        ));
        gun.cooldown = AUTOFIRE_DELAY;
        fx.write(Fx { kind: FxKind::Muzzle(tank.dir), pos: pos + tank.dir.vec() * 2.0 });
        if matches!(tank.team, Team::Player(_)) {
            sfx.write(Sfx::Shoot);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn advance_stops_flush() {
        // Wall at x >= 50: a 16-wide tank centred at x stops at 42.
        let wall = |p: Vec2| p.x + 8.0 > 50.0;
        let p = advance(Vec2::new(40.0, 0.0), Dir::Right, 5.0, wall);
        assert!((p.x - 42.0).abs() < 0.01, "{p}");
        let free = advance(Vec2::new(40.0, 0.0), Dir::Left, 5.0, wall);
        assert_eq!(free.x, 35.0);
    }
}
