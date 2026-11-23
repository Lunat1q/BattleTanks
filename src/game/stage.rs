//! Stage lifecycle: build on enter, enemy spawning, timers, win/lose, pause and controller loss.

use super::*;
use crate::geom::{FIELD, overlaps, rect_at};
use crate::input::{Device, DeviceIntents};
use crate::levels;
use crate::rng::Rng;
use crate::rules::{self, BONUS_CARRIERS, Rank};
use crate::terrain::{Terrain, Tile};
use bevy::window::AppLifecycle;

pub const PLAYER_SPAWNS: [Vec2; 2] = [Vec2::new(72.0, 8.0), Vec2::new(136.0, 8.0)];
pub const ENEMY_SPAWNS: [Vec2; 3] = [Vec2::new(104.0, FIELD - 8.0), Vec2::new(FIELD - 8.0, FIELD - 8.0), Vec2::new(8.0, FIELD - 8.0)];
pub const BASE_POS: Vec2 = Vec2::new(104.0, 8.0);
// Short: the world (and the players' tanks) wait out the STAGE banner.
const INTRO_TIME: f32 = 1.2;

pub fn plugin(app: &mut App) {
    app.init_resource::<super::versus::Match>()
        .add_systems(OnEnter(Screen::Stage), setup)
        .add_systems(OnExit(Screen::Stage), |mut paused: ResMut<Paused>| paused.0 = false)
        .add_systems(Update, (watch_controllers, pause_when_away, toggle_pause, advance_phase).chain().run_if(in_state(Screen::Stage)));
}

/// App lifecycle -> whether the game is now in the background (Home, another app, TV off); `None` = no change.
pub fn backgrounded(e: AppLifecycle) -> Option<bool> {
    match e {
        AppLifecycle::WillSuspend | AppLifecycle::Suspended => Some(true),
        AppLifecycle::WillResume | AppLifecycle::Running => Some(false),
        AppLifecycle::Idle => None,
    }
}

/// Going to the background pauses a live stage, like Start; coming back leaves the pause menu up. In a LAN session
/// the news comes through the lockstep (`Intent::away`), so both machines pause on the same tick.
fn pause_when_away(
    mut lifecycle: MessageReader<AppLifecycle>,
    net: Res<crate::net::Net>,
    devices: Res<DeviceIntents>,
    run: Res<StageRun>,
    mut paused: ResMut<Paused>,
) {
    let away = if net.linked() {
        lifecycle.clear();
        devices.0.iter().any(|(_, i)| i.away)
    } else {
        lifecycle.read().any(|&e| backgrounded(e) == Some(true))
    };
    if away && matches!(run.phase, Phase::Play | Phase::Won) {
        paused.0 = true;
    }
}

fn setup(
    mut commands: Commands,
    mut session: ResMut<Session>,
    mut view: ResMut<View>,
    settings: Res<crate::settings::Settings>,
    duel: Res<super::versus::Match>,
    mut rng: ResMut<Rng>,
    mut sfx: MessageWriter<Sfx>,
    mut feats: MessageWriter<Feat>,
) {
    view.bottom = 0.0;
    // A classic run's stages are logged as they end, so an empty log = its first stage; every district is a run.
    let fresh = session.mode == Mode::City || session.log.is_empty();
    let joined = [0, 1].map(|i| session.slots[i].is_some());
    feats.write(Feat::Start { mode: session.mode, stage: session.stage, joined, fresh });
    let (mut terrain, roster) = if session.mode == Mode::City {
        (super::city::setup(&mut commands, &mut session), Vec::new())
    } else if session.mode == Mode::Versus {
        commands.remove_resource::<super::city::CityRun>();
        super::versus::reset_slots(&mut session);
        commands.insert_resource(super::versus::Armies::new(duel.budget, &mut rng));
        for (side, at) in super::versus::BASES.into_iter().enumerate() {
            commands.spawn((Base(side), Transform::from_translation(at.extend(0.0)), DespawnOnExit(Screen::Stage)));
        }
        (crate::versus_map::load(session.stage), Vec::new())
    } else {
        commands.remove_resource::<super::city::CityRun>();
        let stage = levels::load(session.stage);
        commands.spawn((Base(0), Transform::from_translation(BASE_POS.extend(0.0)), DespawnOnExit(Screen::Stage)));
        (stage.terrain, stage.roster)
    };
    if fresh && settings.easy_mode && session.mode != Mode::Versus {
        for slot in session.slots.iter_mut().flatten() {
            slot.lives += rules::EASY_EXTRA_LIVES;
        }
    }
    // Everything is dirty once so the renderer builds the whole map.
    terrain.dirty = terrain.cells().collect();
    commands.insert_resource(terrain);
    commands.insert_resource(StageRun {
        phase: Phase::Intro,
        timer: INTRO_TIME,
        roster,
        next_enemy: 0,
        spawn_timer: 0.0,
        spawn_point: 0,
        freeze: 0.0,
        frozen: [0.0; 2],
        shovel: [0.0; 2],
        base_alive: [true; 2],
        clock: 0.0,
    });
    let mode = session.mode;
    for (i, slot) in session.slots.iter_mut().enumerate() {
        if let Some(slot) = slot.as_mut() {
            slot.kills = [0; 4];
            slot.big = [0; 3];
            slot.pickups = 0;
            if !slot.out {
                // Already there when the banner lifts: no spawn twinkle to sit through, just the respawn shield.
                let (at, facing) = player_start(mode, i);
                let e = spawn_player(&mut commands, i, slot.level, at, facing);
                commands.entity(e).remove::<Spawning>().insert(Shield(rules::respawn_shield(settings.easy_mode)));
            }
        }
    }
    sfx.write(Sfx::StageStart);
}

/// Where player `slot` starts a stage, and facing which way: beside the base; in VERSUS player 2 at the top.
pub fn player_start(mode: Mode, slot: usize) -> (Vec2, Dir) {
    if mode == Mode::Versus { super::versus::SPAWNS[slot] } else { (PLAYER_SPAWNS[slot], Dir::Up) }
}

pub fn spawn_player(commands: &mut Commands, slot: usize, level: u8, pos: Vec2, facing: Dir) -> Entity {
    commands
        .spawn((
            Tank::new(Team::Player(slot), facing, rules::PLAYER_SPEED),
            Gun { stats: rules::player_gun(level), trigger: false, cooldown: 0.0 },
            Spawning(rules::SPAWN_TIME),
            Transform::from_translation(pos.extend(0.0)),
            DespawnOnExit(Screen::Stage),
        ))
        .id()
}

/// Where a player re-enters: their `start` (`player_start`) raised to the bottom of the view, or the nearest open
/// tank-sized spot beside or above it. On a classic or VERSUS stage that is always the start itself. In City Run
/// (`reach`) only a spot from which the tank can climb on without going below the view (district 7 kept respawning
/// a player in a sealed pocket).
pub fn respawn_point(start: Vec2, bottom: f32, terrain: &Terrain, reach: Option<&super::city::Reach>) -> Vec2 {
    let x0 = start.x;
    let y0 = (bottom / 8.0).ceil() * 8.0 + start.y;
    // Nearest columns first, the whole width; rows up to most of the view.
    let mut cols: Vec<f32> = (1..crate::geom::CELLS).map(|c| c as f32 * 8.0).collect();
    cols.sort_by(|a, b| (a - x0).abs().total_cmp(&(b - x0).abs()));
    (0..20)
        .flat_map(|row| cols.iter().map(move |&x| Vec2::new(x, y0 + row as f32 * 8.0)))
        .find(|&p| !terrain.blocks_tank(rect_at(p, 16.0)) && reach.is_none_or(|r| r.climbs(p)))
        .unwrap_or(Vec2::new(x0, y0))
}

/// One enemy tank; `district` scales City Run ranks (ignored for `Normal`).
pub fn spawn_enemy(commands: &mut Commands, kind: EnemyKind, rank: Rank, pos: Vec2, bonus: bool, district: usize, rng: &mut Rng) -> Entity {
    let hp = rank.hp(kind, district);
    commands
        .spawn((
            Tank { size: rank.size(), ..Tank::new(Team::Enemy, Dir::Down, rank.speed(kind)) },
            Gun {
                stats: GunStats { bullet_speed: kind.bullet_speed(), max_bullets: rank.max_bullets(), power: false },
                trigger: false,
                cooldown: 0.0,
            },
            Enemy {
                kind,
                rank,
                hp,
                max_hp: hp,
                bonus,
                turn_in: rng.range(0.5, 2.0),
                fire_in: rng.range(0.5, 1.5),
                skill: 4.0,
                phase: 0,
                mark: None,
            },
            Spawning(rules::SPAWN_TIME),
            Transform::from_translation(pos.extend(0.0)),
            DespawnOnExit(Screen::Stage),
        ))
        .id()
}

pub fn spawn_enemies(
    mut commands: Commands,
    mut run: ResMut<StageRun>,
    session: Res<Session>,
    time: Res<Time>,
    tanks: Query<&Transform, With<Tank>>,
    enemies: Query<(), With<Enemy>>,
    mut rng: ResMut<Rng>,
) {
    let players = session.players();
    let enemies_alive = enemies.iter().count();
    if run.next_enemy >= run.roster.len() || run.phase != Phase::Play {
        return;
    }
    run.spawn_timer -= time.delta_secs();
    if run.spawn_timer > 0.0 || enemies_alive >= rules::max_enemies(players) {
        return;
    }
    let pos = ENEMY_SPAWNS[run.spawn_point % 3];
    let spot = rect_at(pos, 16.0);
    if tanks.iter().any(|t| overlaps(spot, rect_at(t.translation.xy(), 16.0))) {
        run.spawn_point += 1; // occupied, try the next point next frame
        return;
    }
    let kind = run.roster[run.next_enemy];
    spawn_enemy(&mut commands, kind, Rank::Normal, pos, BONUS_CARRIERS.contains(&run.next_enemy), session.stage, &mut rng);
    run.next_enemy += 1;
    run.spawn_point += 1;
    run.spawn_timer = rules::spawn_interval(session.stage, players);
}

pub fn tick_timers(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<StageRun>,
    mut terrain: ResMut<Terrain>,
    settings: Res<crate::settings::Settings>,
    mut timed: Query<(Entity, &Tank, &mut Gun, Option<&mut Spawning>, Option<&mut Shield>, Option<&mut Paralyzed>, Option<&mut HitFlash>)>,
) {
    let dt = time.delta_secs();
    for (e, tank, mut gun, spawning, shield, para, flash) in &mut timed {
        gun.cooldown -= dt;
        if let Some(mut s) = spawning {
            s.0 -= dt;
            if s.0 <= 0.0 {
                commands.entity(e).remove::<Spawning>();
                if matches!(tank.team, Team::Player(_)) {
                    commands.entity(e).insert(Shield(rules::respawn_shield(settings.easy_mode)));
                }
            }
        }
        if let Some(mut s) = shield {
            s.0 -= dt;
            if s.0 <= 0.0 {
                commands.entity(e).remove::<Shield>();
            }
        }
        if let Some(mut p) = para {
            p.0 -= dt;
            if p.0 <= 0.0 {
                commands.entity(e).remove::<Paralyzed>();
            }
        }
        if let Some(mut f) = flash {
            f.0 -= dt;
            if f.0 <= 0.0 {
                commands.entity(e).remove::<HitFlash>();
            }
        }
    }
    if run.phase == Phase::Play {
        run.clock += dt;
    }
    run.freeze = (run.freeze - dt).max(0.0);
    for side in 0..2 {
        run.frozen[side] = (run.frozen[side] - dt).max(0.0);
        let left = run.shovel[side];
        if left > 0.0 {
            let left = (left - dt).max(0.0);
            run.shovel[side] = left;
            // Steel, blinking back to brick during the last 3 seconds.
            let steel = left > 3.0 || ((left * 4.0) as u32).is_multiple_of(2);
            terrain.set_ring(side, if steel && left > 0.0 { Tile::Steel } else { Tile::Brick });
        }
    }
}

/// A player who has been out `rules::BORROW_WAIT` presses A to take one of the partner's spare tanks (any mode).
pub fn borrow_life(
    mut commands: Commands,
    time: Res<Time>,
    mut session: ResMut<Session>,
    devices: Res<DeviceIntents>,
    run: Res<StageRun>,
    view: Res<View>,
    terrain: Res<Terrain>,
    reach: Option<Res<super::city::Reach>>,
    mut sfx: MessageWriter<Sfx>,
    mut feats: MessageWriter<Feat>,
) {
    if run.phase == Phase::Play {
        for s in session.slots.iter_mut().flatten() {
            s.out_wait -= time.delta_secs();
        }
    }
    // VERSUS: the other player is no partner to borrow from.
    let players = if session.mode == Mode::Versus { 0 } else { 2 };
    for p in 0..players {
        if run.phase == Phase::Play && session.intent(p, &devices).confirm && session.borrow(p) {
            let level = session.slots[p].as_ref().map_or(0, |s| s.level);
            let partner_left = session.slots[1 - p].as_ref().map_or(0, |s| s.lives);
            feats.write(Feat::Borrow { slot: p, partner_left });
            let (start, facing) = player_start(session.mode, p);
            spawn_player(&mut commands, p, level, respawn_point(start, view.bottom, &terrain, reach.as_deref()), facing);
            sfx.write(Sfx::ExtraLife);
        }
    }
}

pub fn check_end(
    mut run: ResMut<StageRun>,
    mut session: ResMut<Session>,
    city: Option<Res<super::city::CityRun>>,
    enemies: Query<(), With<Enemy>>,
    terrain: Res<Terrain>,
    mut duel: ResMut<super::versus::Match>,
    mut sfx: MessageWriter<Sfx>,
    mut feats: MessageWriter<Feat>,
) {
    if run.phase != Phase::Play {
        return;
    }
    if session.mode == Mode::Versus {
        if let Some(winner) = super::versus::outcome(run.base_alive) {
            run.phase = Phase::Won;
            run.timer = rules::END_DELAY;
            duel.last = winner;
            if let Some(w) = winner {
                duel.wins[w] += 1;
            }
            sfx.write(Sfx::BaseBoom);
        }
        return;
    }
    let everyone_out = session.slots.iter().flatten().all(|s| s.out);
    if !run.base_alive[0] || everyone_out {
        run.phase = Phase::Lost;
        run.timer = rules::END_DELAY;
        sfx.write(Sfx::GameOver);
    } else if city.map_or(run.next_enemy >= run.roster.len() && enemies.is_empty(), |c| c.boss_down) {
        run.phase = Phase::Won;
        run.timer = rules::END_DELAY;
        if session.mode == Mode::Classic {
            let bonus = rules::time_bonus(session.stage, session.players(), (run.clock * 10.0) as u32);
            session.bonus = bonus;
            if (0..2).map(|p| session.add_score(p, bonus)).sum::<u32>() > 0 {
                sfx.write(Sfx::ExtraLife);
            }
        }
        let out = [0, 1].map(|i| session.slots[i].as_ref().is_some_and(|s| s.out));
        let ring_gone = session.mode == Mode::Classic && Terrain::base_ring().all(|(x, y)| terrain.get(x, y) == Tile::Empty);
        feats.write(Feat::Won { out, ring_gone });
    }
    if run.phase != Phase::Play && session.mode == Mode::Classic {
        for (slot, s) in session.slots.iter().enumerate() {
            if let Some(s) = s {
                feats.write(Feat::Score { slot, score: s.score });
            }
        }
    }
}

fn advance_phase(
    time: Res<Time>,
    paused: Res<Paused>,
    session: Res<Session>,
    mut run: ResMut<StageRun>,
    mut next: ResMut<NextState<Screen>>,
) {
    if paused.0 || run.phase == Phase::Play {
        return;
    }
    run.timer -= time.delta_secs();
    if run.timer > 0.0 {
        return;
    }
    match run.phase {
        Phase::Intro => run.phase = Phase::Play,
        _ if session.mode == Mode::Versus => next.set(Screen::VersusResults),
        Phase::Won => next.set(Screen::Results),
        Phase::Lost => next.set(Screen::GameOver),
        Phase::Play => {}
    }
}

fn toggle_pause(
    session: Res<Session>,
    devices: Res<DeviceIntents>,
    run: Res<StageRun>,
    mut paused: ResMut<Paused>,
    mut sfx: MessageWriter<Sfx>,
    mut feats: MessageWriter<Feat>,
) {
    let start = (0..2).any(|i| session.intent(i, &devices).start || session.intent(i, &devices).pause);
    let lost = session.slots.iter().flatten().any(|s| !devices.connected(s.device));
    if start && !lost && matches!(run.phase, Phase::Play | Phase::Won) {
        paused.0 = !paused.0;
        sfx.write(Sfx::Pause);
        if let Some(slot) = (0..2).find(|&i| session.intent(i, &devices).start || session.intent(i, &devices).pause)
            && paused.0
        {
            feats.write(Feat::Pause { slot });
        }
    }
}

/// A slot whose controller vanished pauses the game; any free controller pressing A takes it over.
fn watch_controllers(mut session: ResMut<Session>, devices: Res<DeviceIntents>, mut paused: ResMut<Paused>) {
    let used: Vec<Device> = session.slots.iter().flatten().map(|s| s.device).collect();
    let free_press =
        devices.0.iter().find(|(d, i)| matches!(d, Device::Pad(_)) && (i.confirm || i.start) && !used.contains(d)).map(|(d, _)| *d);
    for slot in session.slots.iter_mut().flatten() {
        if !devices.connected(slot.device) {
            paused.0 = true;
            if let Some(d) = free_press {
                slot.device = d;
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaving_the_app_counts_as_background_until_resumed() {
        use AppLifecycle::*;
        assert_eq!(
            [Idle, Running, WillSuspend, Suspended, WillResume].map(backgrounded),
            [None, Some(false), Some(true), Some(true), Some(false)]
        );
    }

    #[test]
    fn an_out_player_can_borrow_a_spare_tank() {
        let slot = |lives, out| Some(Slot { lives, out, ..Slot::new(Device::KeysLeft) });
        let mut s = Session { slots: [slot(0, true), slot(2, false)], ..default() };
        s.slots[0].as_mut().unwrap().out_wait = 0.5;
        assert!(!s.borrow(0), "not before the wait is over");
        s.slots[0].as_mut().unwrap().out_wait = 0.0;
        assert!(!s.can_borrow(1), "player 2 is still in");
        assert!(s.borrow(0));
        assert_eq!((s.slots[0].as_ref().unwrap().out, s.slots[1].as_ref().unwrap().lives), (false, 1));
        assert!(!s.borrow(0), "only while out");
        let mut s = Session { slots: [slot(0, true), slot(0, false)], ..default() };
        assert!(!s.borrow(0), "partner has nothing to spare");
        let mut solo = Session { slots: [slot(0, true), None], ..default() };
        assert!(!solo.borrow(0));
    }

    #[test]
    fn respawn_finds_open_ground_at_the_bottom_of_the_view() {
        let classic = Terrain::default();
        assert_eq!(respawn_point(PLAYER_SPAWNS[0], 0.0, &classic, None), PLAYER_SPAWNS[0]);
        let top = super::super::versus::SPAWNS[1].0;
        assert_eq!(respawn_point(top, 0.0, &crate::versus_map::load(1), None), top, "VERSUS player 2 respawns at the top");
        let mut ter = Terrain::new(crate::geom::SUBS * 3);
        for y in 75..79 {
            for x in 16..20 {
                ter.set(x, y, Tile::Steel); // right where player 1 would land at bottom = 293
            }
        }
        let p = respawn_point(PLAYER_SPAWNS[0], 293.0, &ter, None);
        assert!(p.y >= 304.0 && !ter.blocks_tank(rect_at(p, 16.0)), "{p}");
    }

    #[test]
    fn city_respawns_never_land_where_the_only_way_on_is_down() {
        // Owner's report: district 7 respawned a player in a pocket (water both sides, steel above, brick below at
        // the view's bottom) again and again. Every view a district can show must offer a spot that climbs on.
        let mut old_traps = 0;
        for n in 1..=rules::CITY_DISTRICTS {
            let d = crate::city_map::load(n);
            let reach = super::super::city::Reach::new(&d);
            let mut bottom = 0.0;
            while bottom + crate::geom::FIELD <= d.terrain.height() {
                for start in PLAYER_SPAWNS {
                    let p = respawn_point(start, bottom, &d.terrain, Some(&reach));
                    assert!(reach.climbs(p) && !d.terrain.blocks_tank(rect_at(p, 16.0)), "district {n}, view {bottom}: {p}");
                    old_traps += usize::from(!reach.climbs(respawn_point(start, bottom, &d.terrain, None)));
                }
                bottom += 8.0;
            }
        }
        assert!(old_traps > 0, "the plain picker should hit a pocket somewhere, or this test proves nothing");
    }
}
