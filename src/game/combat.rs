//! Bullet flight (terrain, base, tanks, other bullets) and what a hit does.

use super::stage::{BASE_POS, respawn_point, spawn_player};
use super::*;
use crate::geom::{TILE, overlaps, rect_at};
use crate::rng::Rng;
use crate::settings::Settings;
use crate::terrain::{Impact, Terrain};
use bevy::math::Rect;

const BULLET_SIZE: f32 = 4.0;
/// Bullets move in steps no longer than this, so they can't tunnel through a 4-unit brick.
const BULLET_STEP: f32 = 2.0;

#[derive(Message, Clone, Copy)]
pub struct Hit {
    pub target: Entity,
    pub by: Team,
}

/// Can a bullet from `shooter` hit a tank of `target` (owner excluded elsewhere)?
/// Player-on-player hits (paralyze) only with friendly fire on.
pub fn can_hit(shooter: Team, target: Team, friendly_fire: bool) -> bool {
    match (shooter, target) {
        (Team::Enemy, Team::Enemy) => false,
        (Team::Player(_), Team::Player(_)) => friendly_fire,
        _ => true,
    }
}

/// Do two bullets cancel each other? Only across sides.
pub fn opposing(a: Team, b: Team) -> bool {
    matches!(a, Team::Enemy) != matches!(b, Team::Enemy)
}

pub fn fly_bullets(
    mut commands: Commands,
    time: Res<Time>,
    mut terrain: ResMut<Terrain>,
    mut run: ResMut<StageRun>,
    view: Res<View>,
    base: Query<(), With<Base>>,
    mut bullets: Query<(Entity, &Bullet, &mut Transform)>,
    tanks: Query<(Entity, &Tank, &Transform), (Without<Bullet>, Without<Spawning>)>,
    mut hits: MessageWriter<Hit>,
    mut fx: MessageWriter<Fx>,
    mut sfx: MessageWriter<Sfx>,
    mut rumble: MessageWriter<Rumble>,
    settings: Res<Settings>,
) {
    let dt = time.delta_secs().min(0.05);
    let targets: Vec<(Entity, Team, Rect)> = tanks.iter().map(|(e, t, tf)| (e, t.team, rect_at(tf.translation.xy(), t.size))).collect();
    let base = (!base.is_empty()).then(|| rect_at(BASE_POS, TILE));
    let mut swept: Vec<(Entity, Team, Rect)> = Vec::new();

    for (e, b, mut tf) in &mut bullets {
        let start = tf.translation.xy();
        let total = b.speed * dt;
        let steps = (total / BULLET_STEP).ceil().max(1.0) as usize;
        let mut pos = start;
        let mut gone = false;
        for _ in 0..steps {
            pos += b.dir.vec() * (total / steps as f32);
            let r = rect_at(pos, BULLET_SIZE);
            if let Some(&(t, _, _)) =
                targets.iter().find(|(t, team, tr)| *t != b.owner && can_hit(b.team, *team, settings.friendly_fire) && overlaps(r, *tr))
            {
                hits.write(Hit { target: t, by: b.team });
                gone = true;
                break;
            }
            if base.is_some_and(|b| overlaps(r, b)) {
                if run.base_alive {
                    run.base_alive = false;
                    fx.write(Fx { kind: FxKind::BaseBoom, pos: BASE_POS });
                    sfx.write(Sfx::BaseBoom);
                    rumble.write_batch([Rumble::base_lost(0), Rumble::base_lost(1)]);
                }
                gone = true;
                break;
            }
            // The edges of the view are the edges of the world (City Run scrolls it).
            let off_view = (r.min.y < view.bottom || r.max.y > view.top()).then_some(Impact::Edge);
            if let Some(impact) = off_view.or_else(|| terrain.bullet_hit(r, b.dir, b.power)) {
                let player = matches!(b.team, Team::Player(_));
                match impact {
                    Impact::Brick => {
                        fx.write(Fx { kind: FxKind::Bricks, pos });
                        if player {
                            sfx.write(Sfx::Brick);
                        }
                    }
                    Impact::Steel | Impact::Edge => {
                        fx.write(Fx { kind: FxKind::Sparks, pos });
                        if player {
                            sfx.write(Sfx::Steel);
                        }
                    }
                }
                gone = true;
                break;
            }
        }
        if gone {
            commands.entity(e).try_despawn();
            fx.write(Fx { kind: FxKind::BulletPop, pos });
        } else {
            tf.translation.x = pos.x;
            tf.translation.y = pos.y;
            let r = Rect::from_corners(start, pos);
            let pad = Vec2::splat(BULLET_SIZE / 2.0);
            swept.push((e, b.team, Rect { min: r.min - pad, max: r.max + pad }));
        }
    }

    for (i, a) in swept.iter().enumerate() {
        for b in &swept[i + 1..] {
            if opposing(a.1, b.1) && overlaps(a.2, b.2) {
                commands.entity(a.0).try_despawn();
                commands.entity(b.0).try_despawn();
                fx.write(Fx { kind: FxKind::BulletPop, pos: a.2.center() });
            }
        }
    }
}

pub fn resolve_hits(
    mut commands: Commands,
    mut hits: MessageReader<Hit>,
    mut session: ResMut<Session>,
    mut rng: ResMut<Rng>,
    view: Res<View>,
    terrain: Res<Terrain>,
    mut tanks: Query<(&Tank, &Transform, Option<&mut Enemy>, Has<Shield>)>,
    powerups: Query<Entity, (With<PowerUp>, Without<Stash>)>,
    mut fx: MessageWriter<Fx>,
    mut sfx: MessageWriter<Sfx>,
    mut rumble: MessageWriter<Rumble>,
    settings: Res<crate::settings::Settings>,
) {
    let mut dead: Vec<Entity> = Vec::new();
    let city = session.mode == Mode::City;
    for hit in hits.read() {
        if dead.contains(&hit.target) {
            continue;
        }
        let Ok((tank, tf, enemy, shielded)) = tanks.get_mut(hit.target) else { continue };
        let pos = tf.translation.xy();
        match (tank.team, hit.by, enemy) {
            (Team::Enemy, Team::Player(p), Some(mut enemy)) => {
                let drop_power = |commands: &mut Commands, rng: &mut Rng, sfx: &mut MessageWriter<Sfx>| {
                    powerup::spawn_random(commands, rng, &powerups, sfx, view.bottom, city.then_some(&*terrain));
                };
                if enemy.bonus {
                    enemy.bonus = false;
                    drop_power(&mut commands, &mut rng, &mut sfx);
                }
                enemy.hp = enemy.hp.saturating_sub(1);
                if enemy.hp > 0 {
                    commands.entity(hit.target).insert(HitFlash(0.12));
                    sfx.write(Sfx::ArmorHit);
                    continue;
                }
                dead.push(hit.target);
                commands.entity(hit.target).try_despawn();
                let points = enemy.rank.points(enemy.kind);
                if enemy.rank == Rank::Mini {
                    drop_power(&mut commands, &mut rng, &mut sfx);
                }
                if let Some(slot) = session.slots[p].as_mut() {
                    match enemy.rank {
                        Rank::Normal => slot.kills[enemy.kind.index()] += 1,
                        big => slot.big[big as usize - 1] += 1,
                    }
                }
                if session.add_score(p, points) > 0 {
                    sfx.write(Sfx::ExtraLife);
                }
                let big = matches!(enemy.rank, Rank::Mini | Rank::Boss);
                fx.write(Fx { kind: if big { FxKind::BaseBoom } else { FxKind::TankBoom }, pos });
                fx.write(Fx { kind: FxKind::Points(points), pos });
                sfx.write(if enemy.rank == Rank::Boss { Sfx::BossBoom } else { Sfx::EnemyBoom });
                rumble.write(if big { Rumble::death(p) } else { Rumble::tick(p) });
            }
            (Team::Player(target), Team::Player(_), _) if !shielded => {
                commands.entity(hit.target).insert(Paralyzed(rules::PARALYZE_TIME));
                rumble.write(Rumble::stunned(target));
                sfx.write(Sfx::ArmorHit);
            }
            (Team::Player(p), Team::Enemy, _) if !shielded => {
                dead.push(hit.target);
                commands.entity(hit.target).try_despawn();
                fx.write(Fx { kind: FxKind::TankBoom, pos });
                sfx.write(Sfx::PlayerBoom);
                rumble.write(Rumble::death(p));
                let at = respawn_point(p, view.bottom, &terrain);
                player_lost_tank(&mut commands, &mut session, p, at, settings.keep_power);
            }
            _ => {}
        }
    }
}

/// A player's tank is gone: lose stars (unless KEEP POWER), use a spare tank or drop out.
pub fn player_lost_tank(commands: &mut Commands, session: &mut Session, p: usize, respawn_at: Vec2, keep_power: bool) {
    let Some(slot) = session.slots[p].as_mut() else { return };
    if !keep_power {
        slot.level = 0;
    }
    if slot.lives > 0 {
        slot.lives -= 1;
        spawn_player(commands, p, slot.level, respawn_at);
    } else {
        slot.out = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friendly_fire_rules() {
        for ff in [true, false] {
            assert!(can_hit(Team::Player(0), Team::Enemy, ff));
            assert!(can_hit(Team::Enemy, Team::Player(1), ff));
            assert!(!can_hit(Team::Enemy, Team::Enemy, ff));
        }
        assert!(can_hit(Team::Player(0), Team::Player(1), true), "players paralyze each other");
        assert!(!can_hit(Team::Player(0), Team::Player(1), false), "friendly fire off: bullets pass");
        assert!(opposing(Team::Enemy, Team::Player(0)));
        assert!(!opposing(Team::Player(0), Team::Player(1)));
        assert!(!opposing(Team::Enemy, Team::Enemy));
    }
}
