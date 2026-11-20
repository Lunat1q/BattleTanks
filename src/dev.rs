//! Agent/dev smoke hook: `BT_SHOT=out.png` auto-joins two keyboard players, starts `BT_STAGE`
//! (default 1; 0 = stay on the title, or `BT_SCREEN=settings` / `hiscores` / `city` (district map) / `results` / `gameover` (a
//! made-up classic run, to look at the tally and the name entry)) or City Run
//! district `BT_CITY`, saves a screenshot after `BT_SHOT_AT` seconds (default 7) and quits. `BT_VIEW=<units>` pins the
//! City Run view that far up the map, to look at bastions and bosses. Inert unless `BT_SHOT` is set.

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
    if shot.stage == 0 && shot.city == 0 && *state.get() == Screen::Title {
        match std::env::var("BT_SCREEN").as_deref() {
            Ok("settings") => next.set(Screen::Settings),
            Ok("hiscores") => next.set(Screen::HiScores),
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
