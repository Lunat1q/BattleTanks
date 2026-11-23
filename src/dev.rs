//! Agent/dev smoke hook: `BT_SHOT=out.png` auto-joins two keyboard players, starts `BT_STAGE`
//! (default 1; 0 = stay on the title, or `BT_SCREEN=joined` (the title with 1P joined) / `stages` (classic tiles, 9 open) /
//! `settings` / `hiscores` / `city` (district map) / `results` / `gameover` (a
//! made-up classic run, to look at the tally and the name entry)) or City Run
//! district `BT_CITY`, saves a screenshot after `BT_SHOT_AT` seconds (default 7) and quits. `BT_VIEW=<units>` pins the
//! City Run view that far up the map, to look at bastions and bosses. `BT_SCREEN=lan` the LAN CO-OP screen, `bunker` the secret bunker (`BT_TAB=1` OST, `2` sound test). In a stage,
//! `BT_PAUSE=1` opens the pause menu and `BT_FLYOUT=<secs>` fakes two unlocks that long before the shot (the achievement stars).
//! Inert unless `BT_SHOT` is set.

use crate::game::{Mode, Screen, Session, Slot, StageLog, View};
use crate::input::Device;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

#[derive(Resource)]
struct Shot {
    path: String,
    stage: usize,
    city: usize,
    view: Option<f32>,
    at: f32,
    t: f32,
    taken: bool,
    frames: u32,
}

pub fn plugin(app: &mut App) {
    let Ok(path) = std::env::var("BT_SHOT") else { return };
    let num = |k: &str, d: f32| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    let city = num("BT_CITY", 0.0) as usize;
    let stage = if city > 0 { 0 } else { num("BT_STAGE", 1.0) as usize };
    let view = std::env::var("BT_VIEW").ok().and_then(|v| v.parse().ok());
    // `BT_ACHIEVE=1`: achievements on, the host calls only logged, to look at the menu and the run summary.
    if std::env::var("BT_ACHIEVE").is_ok() {
        app.world_mut().resource_mut::<crate::Platform>().achieve = Some(|k, n| info!("achieve: {k} {n}"));
    }
    app.insert_resource(Shot { path, stage, city, view, at: num("BT_SHOT_AT", 7.0), t: 0.0, taken: false, frames: 0 })
        .add_systems(Update, run);
}

fn run(
    mut commands: Commands,
    time: Res<Time>,
    mut shot: ResMut<Shot>,
    mut session: ResMut<Session>,
    state: Res<State<Screen>>,
    mut next: ResMut<NextState<Screen>>,
    mut exit: MessageWriter<AppExit>,
    mut view: ResMut<View>,
    mut rec: ResMut<crate::achieve::Record>,
    mut progress: ResMut<crate::game::progress::Progress>,
    mut paused: ResMut<crate::game::Paused>,
) {
    shot.t += time.delta_secs();
    shot.frames += 1;
    if shot.stage > 0 && *state.get() == Screen::Title && session.players() == 0 {
        session.slots = [Some(Slot::new(Device::KeysLeft)), Some(Slot::new(Device::KeysRight))];
        session.stage = shot.stage;
        next.set(Screen::Stage);
    }
    let keys = || [Some(Slot::new(Device::KeysLeft)), Some(Slot::new(Device::KeysRight))];
    if shot.city > 0 && *state.get() == Screen::Title && session.players() == 0 {
        session.slots = keys();
        session.mode = Mode::City;
        session.stage = shot.city;
        next.set(Screen::Stage);
    }
    if let Some(v) = shot.view.filter(|_| *state.get() == Screen::Stage) {
        view.bottom = v;
    }
    let lead = std::env::var("BT_FLYOUT").ok().and_then(|v| v.parse::<f32>().ok());
    if let Some(lead) = lead.filter(|&l| *state.get() == Screen::Stage && shot.t > shot.at - l && rec.news.is_empty()) {
        info!("smoke: two unlocks {lead} s before the shot");
        rec.news = vec!["first_blood", "stage_1"];
    }
    if *state.get() == Screen::Stage && shot.t > shot.at - 0.9 && std::env::var("BT_PAUSE").is_ok() {
        paused.0 = true;
    }
    if shot.stage == 0 && shot.city == 0 && *state.get() == Screen::Title {
        match std::env::var("BT_SCREEN").as_deref() {
            Ok("settings") => next.set(Screen::Settings),
            Ok("hiscores") => next.set(Screen::HiScores),
            Ok("lan") => next.set(Screen::Lan),
            Ok("bunker") => next.set(Screen::Bunker),
            Ok("joined") if session.players() == 0 => session.slots[0] = Some(Slot::new(Device::KeysLeft)),
            Ok("stages") if session.players() == 0 => {
                // Shown only, not saved.
                session.slots = keys();
                progress.classic = 9;
                next.set(Screen::StageSelect);
            }
            Ok("unlocks") if session.players() == 0 => {
                // A made-up run's unlocks, shown only (not saved).
                session.slots = keys();
                let news = ["first_blood", "stage_1", "flawless", "city_3", "kills_100", "own_goal", "treasure_hunter"];
                rec.done.extend(news);
                rec.news = news.to_vec();
                next.set(Screen::Unlocks);
            }
            Ok("achievements") => {
                // Shown only, not saved: nothing is marked for the host.
                rec.done.extend(["first_blood", "stage_1", "flawless", "own_goal", "city_1", "digger", "kills_100", "streak_10"]);
                rec.steps.extend([("kills_100", 64), ("kills_1000", 64), ("kills_5000", 64), ("wallbreaker", 12)]);
                next.set(Screen::Achievements);
            }
            Ok("play") if session.players() == 0 => {
                session.slots = keys();
                next.set(Screen::Play);
            }
            Ok("city") if session.players() == 0 => {
                session.slots = keys();
                next.set(Screen::CityMap);
            }
            Ok(s @ ("results" | "gameover" | "cityresults" | "board")) if session.players() == 0 => {
                session.slots = keys();
                for (i, slot) in session.slots.iter_mut().flatten().enumerate() {
                    slot.kills = [7 - 3 * i as u32, 4, 2, 1 + i as u32];
                    slot.score = 12_340 + 2_210 * i as u32;
                }
                let won = s != "gameover";
                let city = s == "cityresults" || s == "board";
                if city {
                    session.mode = Mode::City;
                }
                (session.stage, session.bonus) = (4, 1_610);
                session.log = vec![StageLog {
                    stage: 4,
                    tenths: 1_017,
                    won,
                    kills: vec![[7, 4, 2, 1], [4, 4, 2, 2]],
                    pickups: vec![1, 0],
                    big: if city { vec![[14, 2, 1], [6, 1, 0]] } else { vec![] },
                }];
                // `board`: the name entry only; nothing is sent, since the smoke run presses nothing.
                next.set(match s {
                    "board" => Screen::Board,
                    _ if won => Screen::Results,
                    _ => Screen::GameOver,
                });
            }
            _ => {}
        }
    }
    if shot.t > shot.at && !shot.taken {
        shot.taken = true;
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(shot.path.clone()));
    }
    if shot.t > shot.at + 1.5 {
        info!("smoke: {:.0} fps average", shot.frames as f32 / shot.t);
        exit.write(AppExit::Success);
    }
}

/// LAN smoke, two copies and no player: `BT_LAN=host` / `BT_LAN=join` opens LAN CO-OP and hosts / joins the first
/// game heard; `BT_LAN_BOT=1` plays made-up input; `BT_LAN_SECS=n` quits n seconds after linking; `BT_DESYNC=t` (on
/// the joiner) nudges a tank at tick t to test the resync. Each copy logs `lan: tick <t> hash <h>` lines to compare.
pub fn lan_plugin(app: &mut App) {
    if std::env::var("BT_LAN").is_ok() {
        app.add_systems(Update, lan_auto);
    }
}

fn env_num(k: &str) -> Option<u64> {
    std::env::var(k).ok()?.parse().ok()
}

fn lan_auto(
    state: Res<State<Screen>>,
    mut next: ResMut<NextState<Screen>>,
    mut net: ResMut<crate::net::Net>,
    mut linked_at: Local<Option<std::time::Instant>>,
    mut exit: MessageWriter<AppExit>,
) {
    let host = std::env::var("BT_LAN").as_deref() == Ok("host");
    if let Some(t) = *linked_at {
        if env_num("BT_LAN_SECS").is_some_and(|s| t.elapsed().as_secs() >= s) {
            info!("lan: smoke done");
            exit.write(AppExit::Success);
        }
        return;
    }
    if net.linked() {
        *linked_at = Some(std::time::Instant::now());
        return;
    }
    match state.get() {
        Screen::Title => next.set(Screen::Lan),
        Screen::Lan if host && !net.hosting() => net.host(),
        // The host's copy may still hold the beacon port for a moment at start-up: keep trying.
        Screen::Lan if !host && net.finder.is_none() => net.listen(),
        Screen::Lan if !host && !net.joining() && net.finder.as_ref().is_some_and(|f| !f.found.is_empty()) => net.join(0),
        _ => {}
    }
}

fn lan_env(k: &'static str) -> bool {
    static SEEN: std::sync::Mutex<Vec<(&str, bool)>> = std::sync::Mutex::new(Vec::new());
    let mut seen = SEEN.lock().unwrap();
    if let Some(&(_, v)) = seen.iter().find(|(n, _)| *n == k) {
        return v;
    }
    let v = std::env::var(k).is_ok();
    seen.push((k, v));
    v
}

/// The LAN smoke logs hashes.
pub fn lan_log() -> bool {
    lan_env("BT_LAN")
}

/// `BT_LAN_BOT`: made-up input for player `me`, one call per frame. A every 1.5 s (joins, starts, moves on), a new
/// heading every half second, mostly upwards, and fire half the time but only facing up, so it doesn't shoot its own
/// base and the stages last. Never Start, so it doesn't pause.
pub fn lan_bot(me: Option<u8>) -> Option<crate::input::Intent> {
    use std::sync::atomic::{AtomicU32, Ordering};
    if !lan_env("BT_LAN_BOT") {
        return None;
    }
    static FRAME: AtomicU32 = AtomicU32::new(0);
    let n = FRAME.fetch_add(1, Ordering::Relaxed);
    let mut rng = crate::rng::Rng::new((n / 30) as u64 * 31 + me? as u64 + 1);
    use crate::geom::Dir;
    let dir = rng.chance(0.8).then(|| rng.pick(&[Dir::Up, Dir::Up, Dir::Left, Dir::Right, Dir::Down]));
    let up = dir == Some(Dir::Up);
    Some(crate::input::Intent {
        dir,
        fire: up && n % 40 < 20,
        fire_pressed: up && n.is_multiple_of(40),
        confirm: n % 90 == 45,
        ..default()
    })
}

/// `BT_DESYNC=<tick>` on the joiner: shift player 1's tank a little before that tick, as a desync would.
pub fn lan_nudge(world: &mut World, me: u8, tick: u32) {
    if me != 1 || env_num("BT_DESYNC") != Some(tick as u64) {
        return;
    }
    let mut q = world.query::<(&crate::game::Tank, &mut Transform)>();
    if let Some((_, mut tf)) = q.iter_mut(world).find(|(t, _)| t.team == crate::game::Team::Player(0)) {
        tf.translation.x += 0.5;
        warn!("lan: nudged player 1 at tick {tick} (BT_DESYNC)");
    }
}
