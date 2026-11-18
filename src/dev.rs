//! Agent/dev smoke hook: `BT_SHOT=out.png` auto-joins two keyboard players, starts `BT_STAGE`
//! (default 1; 0 = stay on the title), saves a screenshot after `BT_SHOT_AT` seconds (default 7) and quits.
//! Inert unless the env var is set.

use crate::game::{Screen, Session, Slot};
use crate::input::Device;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

#[derive(Resource)]
struct Shot {
    path: String,
    stage: usize,
    at: f32,
    t: f32,
    taken: bool,
}

pub fn plugin(app: &mut App) {
    let Ok(path) = std::env::var("BT_SHOT") else { return };
    let num = |k: &str, d: f32| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    app.insert_resource(Shot { path, stage: num("BT_STAGE", 1.0) as usize, at: num("BT_SHOT_AT", 7.0), t: 0.0, taken: false })
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
) {
    shot.t += time.delta_secs();
    if shot.stage > 0 && *state.get() == Screen::Title && session.players() == 0 {
        session.slots = [Some(Slot::new(Device::KeysLeft)), Some(Slot::new(Device::KeysRight))];
        session.stage = shot.stage;
        next.set(Screen::Stage);
    }
    if shot.t > shot.at && !shot.taken {
        shot.taken = true;
        commands.spawn(Screenshot::primary_window()).observe(save_to_disk(shot.path.clone()));
    }
    if shot.t > shot.at + 1.5 {
        exit.write(AppExit::Success);
    }
}
