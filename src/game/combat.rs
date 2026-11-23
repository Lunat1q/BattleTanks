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
    /// The tank that fired (for achievements: who destroyed whom).
    pub shooter: Entity,
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
    mut bullets: Query<(Entity, &mut Bullet, &mut Transform)>,
    tanks: Query<(Entity, &Tank, &Transform), (Without<Bullet>, Without<Spawning>)>,
    mut hits: MessageWriter<Hit>,
    mut fx: MessageWriter<Fx>,
    mut sfx: MessageWriter<Sfx>,
    mut rumble: MessageWriter<Rumble>,
    settings: Res<Settings>,
    ids: Ids,
    mut feats: MessageWriter<Feat>,
) {
    let dt = time.delta_secs().min(0.05);
    // Spawn order throughout: the first tank a shell touches takes the hit, and earlier shells chip bricks first.
    let targets: Vec<(Entity, Team, Rect)> = tanks
        .iter()
        .sort_by_key::<Entity, _>(|e| order(&ids, *e))
        .map(|(e, t, tf)| (e, t.team, rect_at(tf.translation.xy(), t.size)))
        .collect();
    let base = (!base.is_empty()).then(|| rect_at(BASE_POS, TILE));
    let mut swept: Vec<(Entity, Team, Rect)> = Vec::new();

    for (e, mut b, mut tf) in bullets.iter_mut().sort_by_key::<Entity, _>(|e| order(&ids, *e)) {
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
                hits.write(Hit { target: t, by: b.team, shooter: b.owner });
                gone = true;
                break;
            }
            if base.is_some_and(|b| overlaps(r, b)) {
                if let (true, Team::Player(p)) = (run.base_alive, b.team) {
                    feats.write(Feat::OwnBase { slot: p });
                }
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
            let player = matches!(b.team, Team::Player(_));
            if let Some(impact) = off_view.or_else(|| terrain.bullet_hit(r, b.dir, b.power, player)) {
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
            if b.trees && terrain.cut_trees(r, b.dir) {
                b.trees = false;
                fx.write(Fx { kind: FxKind::Leaves, pos });
                sfx.write(Sfx::Brick);
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
                if let Team::Player(p) = if a.1 == Team::Enemy { b.1 } else { a.1 } {
                    feats.write(Feat::Cancel { slot: p });
                }
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
    mut tanks: Query<(&Tank, &Transform, Option<&mut Enemy>, Has<Shield>, Has<Paralyzed>, Has<Armour>)>,
    powerups: Query<Entity, (With<PowerUp>, Without<Stash>)>,
    mut fx: MessageWriter<Fx>,
    mut sfx: MessageWriter<Sfx>,
    mut rumble: MessageWriter<Rumble>,
    settings: Res<crate::settings::Settings>,
    mut feats: MessageWriter<Feat>,
) {
    let mut dead: Vec<Entity> = Vec::new();
    // This frame's (player, enemy) kills and deaths, for Mutual Destruction.
    let (mut kills, mut deaths) = (Vec::new(), Vec::new());
    let city = session.mode == Mode::City;
    for hit in hits.read() {
        if dead.contains(&hit.target) {
            continue;
        }
        let Ok((tank, tf, enemy, shield, stunned, armour)) = tanks.get_mut(hit.target) else { continue };
        let shielded = shield || armour;
        let pos = tf.translation.xy();
        match (tank.team, hit.by, enemy) {
            (Team::Enemy, Team::Player(p), Some(mut enemy)) => {
                let drop_power = |commands: &mut Commands, rng: &mut Rng, sfx: &mut MessageWriter<Sfx>| {
                    powerup::spawn_random(commands, rng, &powerups, sfx, view.bottom, &terrain, city);
                };
                if enemy.bonus {
                    enemy.bonus = false;
                    drop_power(&mut commands, &mut rng, &mut sfx);
                }
                enemy.hp = enemy.hp.saturating_sub(1);
                if enemy.hp > 0 {
                    commands.entity(hit.target).insert(HitFlash(0.12));
                    sfx.write(Sfx::ArmorHit);
                    rumble.write(Rumble::hit(p));
                    continue;
                }
                dead.push(hit.target);
                commands.entity(hit.target).try_despawn();
                kills.push((p, hit.target));
                feats.write(Feat::Kill { slot: p, kind: enemy.kind, rank: enemy.rank });
                let points = enemy.rank.points(enemy.kind);
                if enemy.rank == Rank::Mini {
                    drop_power(&mut commands, &mut rng, &mut sfx);
                }
                if let Some(slot) = session.slots[p].as_mut() {
                    match enemy.rank {
                        Rank::Normal => slot.kills[enemy.kind.index()] += 1,
                        // Called in by the boss, any number: not in the district's facts the board checks.
                        Rank::Sentry => {}
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
                rumble.write(if big { Rumble::big_kill(p) } else { Rumble::kill(p) });
            }
            (Team::Player(target), Team::Player(by), _) if !shielded => {
                feats.write(Feat::Stun { by });
                commands.entity(hit.target).insert(Paralyzed(rules::PARALYZE_TIME));
                rumble.write(Rumble::stunned(target));
                sfx.write(Sfx::ArmorHit);
            }
            (Team::Player(p), Team::Enemy, _) if !shielded && rules::armour_holds(session.slots[p].as_ref().map_or(0, |s| s.level)) => {
                commands.entity(hit.target).insert(Armour(rules::ARMOUR_TIME));
                fx.write(Fx { kind: FxKind::Sparks, pos });
                sfx.write(Sfx::ArmorHit);
                rumble.write(Rumble::stunned(p));
            }
            (Team::Player(p), Team::Enemy, _) if !shielded => {
                dead.push(hit.target);
                commands.entity(hit.target).try_despawn();
                fx.write(Fx { kind: FxKind::TankBoom, pos });
                sfx.write(Sfx::PlayerBoom);
                rumble.write_batch(Rumble::death(p));
                feats.write(Feat::Lost { slot: p, stunned });
                deaths.push((p, hit.shooter));
                let at = respawn_point(p, view.bottom, &terrain);
                player_lost_tank(&mut commands, &mut session, p, at, settings.easy_mode);
            }
            _ => {}
        }
    }
    for d in deaths.into_iter().filter(|d| kills.contains(d)) {
        feats.write(Feat::Mutual { slot: d.0 });
    }
}

/// Spent level-4 armour runs out: the stars drop as for a lost tank, and a tank still on water sinks.
pub fn armour_wears_off(
    mut commands: Commands,
    time: Res<Time>,
    mut session: ResMut<Session>,
    view: Res<View>,
    terrain: Res<Terrain>,
    settings: Res<crate::settings::Settings>,
    mut tanks: Query<(Entity, &Tank, &Transform, &mut Gun, &mut Armour)>,
    mut fx: MessageWriter<Fx>,
    mut sfx: MessageWriter<Sfx>,
    mut rumble: MessageWriter<Rumble>,
    mut feats: MessageWriter<Feat>,
    ids: Ids,
) {
    // Spawn order: two tanks sinking on one tick take spare tanks and respawn alike on every machine.
    for (e, tank, tf, mut gun, mut armour) in tanks.iter_mut().sort_by_key::<Entity, _>(|e| order(&ids, *e)) {
        armour.0 -= time.delta_secs();
        let Team::Player(p) = tank.team else { continue };
        if armour.0 > 0.0 {
            continue;
        }
        commands.entity(e).remove::<Armour>();
        let Some(slot) = session.slots[p].as_mut() else { continue };
        slot.level = rules::level_after_loss(slot.level, settings.easy_mode);
        gun.stats = rules::player_gun(slot.level);
        let pos = tf.translation.xy();
        if terrain.blocks_tank(rect_at(pos, tank.size)) {
            commands.entity(e).try_despawn();
            fx.write(Fx { kind: FxKind::TankBoom, pos });
            sfx.write(Sfx::PlayerBoom);
            rumble.write_batch(Rumble::death(p));
            feats.write(Feat::Lost { slot: p, stunned: false });
            let at = respawn_point(p, view.bottom, &terrain);
            player_lost_tank(&mut commands, &mut session, p, at, settings.easy_mode);
        }
    }
}

/// A player's tank is gone: lose stars (one in EASY MODE, else all), use a spare tank or drop out.
pub fn player_lost_tank(commands: &mut Commands, session: &mut Session, p: usize, respawn_at: Vec2, easy_mode: bool) {
    let Some(slot) = session.slots[p].as_mut() else { return };
    slot.level = rules::level_after_loss(slot.level, easy_mode);
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
