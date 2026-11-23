//! Bullet flight (terrain, base, tanks, other bullets) and what a hit does.

use super::stage::{player_start, respawn_point, spawn_player};
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
/// Player-on-player hits (paralyze) only with friendly fire on. In VERSUS sides always hit each other and never
/// themselves, whatever FRIENDLY FIRE says.
pub fn can_hit(shooter: Team, target: Team, friendly_fire: bool, versus: bool) -> bool {
    if versus {
        return shooter.side() != target.side();
    }
    match (shooter, target) {
        (Team::Enemy, Team::Enemy) => false,
        (Team::Player(_), Team::Player(_)) => friendly_fire,
        _ => true,
    }
}

/// Do two bullets cancel each other? Only across sides (players against enemies; in VERSUS, side against side).
pub fn opposing(a: Team, b: Team, versus: bool) -> bool {
    if versus { a.side() != b.side() } else { matches!(a, Team::Enemy) != matches!(b, Team::Enemy) }
}

/// Does a bullet of `team` damage the base of `side`? In classic any bullet does (even a player's); in VERSUS only
/// the other side's, so nobody loses by shooting their own base.
pub fn hurts_base(team: Team, side: usize, versus: bool) -> bool {
    !versus || team.side() != Some(side)
}

pub fn fly_bullets(
    mut commands: Commands,
    time: Res<Time>,
    mut terrain: ResMut<Terrain>,
    mut run: ResMut<StageRun>,
    view: Res<View>,
    session: Res<Session>,
    bases: Query<(&Base, &Transform), Without<Bullet>>,
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
    let versus = session.mode == Mode::Versus;
    let bases: Vec<(usize, Vec2)> = bases.iter().map(|(b, tf)| (b.0, tf.translation.xy())).collect();
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
            if let Some(&(t, _, _)) = targets
                .iter()
                .find(|(t, team, tr)| *t != b.owner && can_hit(b.team, *team, settings.friendly_fire, versus) && overlaps(r, *tr))
            {
                hits.write(Hit { target: t, by: b.team, shooter: b.owner });
                gone = true;
                break;
            }
            if let Some(&(side, at)) = bases.iter().find(|(_, at)| overlaps(r, rect_at(*at, TILE))) {
                if run.base_alive[side] && hurts_base(b.team, side, versus) {
                    if let Team::Player(p) = b.team {
                        feats.write(Feat::OwnBase { slot: p });
                    }
                    run.base_alive[side] = false;
                    fx.write(Fx { kind: FxKind::BaseBoom, pos: at });
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
            if opposing(a.1, b.1, versus) && overlaps(a.2, b.2) {
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
    reach: Option<Res<super::city::Reach>>,
    mut tanks: Query<(&Tank, &Transform, Option<&mut Enemy>, Has<Shield>, Has<Paralyzed>, Has<Armour>)>,
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
    let versus = session.mode == Mode::Versus;
    for hit in hits.read() {
        if dead.contains(&hit.target) {
            continue;
        }
        let Ok((tank, tf, enemy, shield, stunned, armour)) = tanks.get_mut(hit.target) else { continue };
        let shielded = shield || armour;
        let pos = tf.translation.xy();
        match (tank.team, hit.by, enemy) {
            (Team::Enemy, Team::Player(p), Some(mut enemy)) => {
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
                // On the kill, not the first hit (owner: a HEAVY dropped its power-up on the first of four hits).
                if enemy.bonus || enemy.rank == Rank::Mini {
                    powerup::spawn_random(&mut commands, &mut rng, &mut sfx, view.bottom, &terrain, city);
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
            (Team::Player(target), Team::Player(by), _) if !shielded && !versus => {
                feats.write(Feat::Stun { by });
                commands.entity(hit.target).insert(Paralyzed(rules::PARALYZE_TIME));
                rumble.write(Rumble::stunned(target));
                sfx.write(Sfx::ArmorHit);
            }
            // VERSUS: the other player's shell counts like an enemy's (armour first, then the tank).
            (Team::Player(p), by, _)
                if (by == Team::Enemy || versus) && !shielded && rules::armour_holds(session.slots[p].as_ref().map_or(0, |s| s.level)) =>
            {
                commands.entity(hit.target).insert(Armour(rules::ARMOUR_TIME));
                fx.write(Fx { kind: FxKind::Sparks, pos });
                sfx.write(Sfx::ArmorHit);
                rumble.write(Rumble::stunned(p));
            }
            (Team::Player(p), by, _) if (by == Team::Enemy || versus) && !shielded => {
                dead.push(hit.target);
                if let Team::Player(k) = by
                    && let Some(s) = session.slots[k].as_mut()
                {
                    s.frags += 1;
                }
                commands.entity(hit.target).try_despawn();
                fx.write(Fx { kind: FxKind::TankBoom, pos });
                sfx.write(Sfx::PlayerBoom);
                rumble.write_batch(Rumble::death(p));
                feats.write(Feat::Lost { slot: p, stunned });
                deaths.push((p, hit.shooter));
                let at = respawn_point(player_start(session.mode, p).0, view.bottom, &terrain, reach.as_deref());
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
    reach: Option<Res<super::city::Reach>>,
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
            let at = respawn_point(player_start(session.mode, p).0, view.bottom, &terrain, reach.as_deref());
            player_lost_tank(&mut commands, &mut session, p, at, settings.easy_mode);
        }
    }
}

/// A player's tank is gone: lose stars (one in EASY MODE, else all), use a spare tank or drop out.
pub fn player_lost_tank(commands: &mut Commands, session: &mut Session, p: usize, respawn_at: Vec2, easy_mode: bool) {
    let facing = player_start(session.mode, p).1;
    let versus = session.mode == Mode::Versus;
    let Some(slot) = session.slots[p].as_mut() else { return };
    slot.level = rules::level_after_loss(slot.level, easy_mode);
    if versus {
        super::versus::lost_tank(slot);
    } else if slot.lives > 0 {
        slot.lives -= 1;
        spawn_player(commands, p, slot.level, respawn_at, facing);
    } else {
        slot.out = true;
        slot.out_wait = rules::BORROW_WAIT;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friendly_fire_rules() {
        for ff in [true, false] {
            assert!(can_hit(Team::Player(0), Team::Enemy, ff, false));
            assert!(can_hit(Team::Enemy, Team::Player(1), ff, false));
            assert!(!can_hit(Team::Enemy, Team::Enemy, ff, false));
            assert!(can_hit(Team::Player(0), Team::Player(1), ff, true), "VERSUS: sides always hit");
            assert!(!can_hit(Team::Player(1), Team::Player(1), ff, true));
        }
        assert!(can_hit(Team::Player(0), Team::Player(1), true, false), "players paralyze each other");
        assert!(!can_hit(Team::Player(0), Team::Player(1), false, false), "friendly fire off: bullets pass");
        assert!(opposing(Team::Enemy, Team::Player(0), false));
        assert!(!opposing(Team::Player(0), Team::Player(1), false));
        assert!(opposing(Team::Player(0), Team::Player(1), true), "VERSUS shells cancel across sides");
        assert!(!opposing(Team::Enemy, Team::Enemy, false));
    }

    #[test]
    fn versus_bases_take_damage_only_from_the_other_side() {
        assert!(hurts_base(Team::Player(0), 0, false), "classic: your own shell still loses the base");
        assert!(hurts_base(Team::Enemy, 0, false));
        assert!(!hurts_base(Team::Player(0), 0, true));
        assert!(hurts_base(Team::Player(1), 0, true));
        assert!(hurts_base(Team::Player(0), 1, true));
    }

    /// Power-ups on the field after `hits` player shells on a flashing HEAVY (4 HP), with one drop already lying there.
    fn drops_after(hits: usize) -> (usize, bool) {
        use bevy::ecs::system::RunSystemOnce;
        let mut app = App::new();
        app.add_message::<Hit>().add_message::<Fx>().add_message::<Sfx>().add_message::<Rumble>().add_message::<Feat>();
        app.insert_resource(Session::default())
            .insert_resource(Rng::new(1))
            .insert_resource(View::default())
            .insert_resource(Terrain::default())
            .insert_resource(crate::settings::Settings::default());
        let world = app.world_mut();
        let old = world.spawn((PowerUp(rules::PowerKind::Star), Transform::default())).id();
        let enemy = Enemy {
            kind: EnemyKind::Armor,
            rank: Rank::Normal,
            hp: 4,
            max_hp: 4,
            bonus: true,
            turn_in: 1.0,
            fire_in: 1.0,
            skill: 0.0,
            phase: 0,
            mark: None,
        };
        let heavy = world.spawn((Tank::new(Team::Enemy, Dir::Down, 32.0), enemy, Transform::default())).id();
        for _ in 0..hits {
            world.write_message(Hit { target: heavy, by: Team::Player(0), shooter: heavy });
        }
        world.run_system_once(resolve_hits).unwrap();
        (world.query::<&PowerUp>().iter(world).count(), world.get_entity(old).is_ok())
    }

    #[test]
    fn a_flashing_heavy_drops_on_the_kill_and_leaves_earlier_drops() {
        // Owner: a HEAVY dropped its power-up on the first hit, and a second drop took away the first.
        assert_eq!(drops_after(3), (1, true), "nothing until it dies");
        assert_eq!(drops_after(4), (2, true), "the kill drops one, the old one stays");
    }

    #[test]
    fn a_versus_hit_destroys_the_other_player_where_classic_only_stuns() {
        use bevy::ecs::system::RunSystemOnce;
        for mode in [Mode::Classic, Mode::Versus] {
            let mut app = App::new();
            app.add_message::<Hit>().add_message::<Fx>().add_message::<Sfx>().add_message::<Rumble>().add_message::<Feat>();
            let slots = [Some(Slot::new(crate::input::Device::KeysLeft)), Some(Slot::new(crate::input::Device::KeysRight))];
            app.insert_resource(Session { slots, mode, ..default() })
                .insert_resource(Rng::new(1))
                .insert_resource(View::default())
                .insert_resource(crate::versus_map::load(1))
                .insert_resource(crate::settings::Settings::default());
            let world = app.world_mut();
            let p2 = world.spawn((Tank::new(Team::Player(1), Dir::Down, 40.0), Transform::default())).id();
            let p1 = world.spawn((Tank::new(Team::Player(0), Dir::Up, 40.0), Transform::default())).id();
            world.write_message(Hit { target: p2, by: Team::Player(0), shooter: p1 });
            world.run_system_once(resolve_hits).unwrap();
            let s = world.resource::<Session>();
            let (frags, lives) = (s.slots[0].as_ref().unwrap().frags, s.slots[1].as_ref().unwrap().lives);
            let respawned: Vec<Vec2> = world
                .query::<(&Tank, &Transform)>()
                .iter(world)
                .filter(|(t, _)| t.team == Team::Player(1))
                .map(|(_, tf)| tf.translation.xy())
                .collect();
            if mode == Mode::Versus {
                let s = &world.resource::<Session>().slots[1].as_ref().unwrap();
                assert_eq!((frags, lives, s.deaths, s.out), (1, rules::START_LIVES, 1, true), "no spare spent: waits to respawn");
                assert!(respawned.is_empty(), "not back before the wait (versus::respawn)");
            } else {
                assert_eq!((frags, lives), (0, rules::START_LIVES), "classic: paralyzed, not destroyed");
                assert!(world.get::<Paralyzed>(p2).is_some());
            }
        }
    }
}
