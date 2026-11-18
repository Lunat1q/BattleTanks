// No console window in release builds (launched from Steam / TV).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Bevy systems naturally take many params with long query types.
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

mod audio;
mod dev;
mod game;
mod geom;
mod gfx;
mod input;
mod levels;
mod rng;
mod rules;
mod terrain;
mod ui;

use bevy::prelude::*;
use bevy::window::{CursorOptions, MonitorSelection, PresentMode, WindowMode};

fn main() {
    // `--windowed` for desk testing; the TV build runs borderless fullscreen.
    let windowed = std::env::args().any(|a| a == "--windowed");
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Battle Tanks".into(),
                mode: if windowed { WindowMode::Windowed } else { WindowMode::BorderlessFullscreen(MonitorSelection::Primary) },
                resolution: (1600, 900).into(),
                present_mode: PresentMode::AutoVsync,
                ..default()
            }),
            primary_cursor_options: Some(CursorOptions { visible: false, ..default() }),
            ..default()
        }))
        .add_plugins((gfx::plugin, audio::plugin, input::plugin, game::plugin, ui::plugin, dev::plugin))
        .add_systems(Update, toggle_fullscreen)
        .run();
}

/// F11 / Alt+Enter toggles fullscreen.
fn toggle_fullscreen(keys: Res<ButtonInput<KeyCode>>, mut windows: Query<&mut Window>) {
    let alt_enter = keys.pressed(KeyCode::AltLeft) && keys.just_pressed(KeyCode::Enter);
    if !(keys.just_pressed(KeyCode::F11) || alt_enter) {
        return;
    }
    for mut w in &mut windows {
        w.mode = match w.mode {
            WindowMode::Windowed => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
            _ => WindowMode::Windowed,
        };
    }
}
