// No console window in release builds (launched from Steam / TV).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// Bevy systems naturally take many params with long query types.
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

mod audio;
mod city_map;
mod dev;
mod game;
mod geom;
mod gfx;
mod input;
mod levels;
mod rng;
mod rules;
mod settings;
mod terrain;
mod ui;

use bevy::prelude::*;
use bevy::window::CursorOptions;

fn main() {
    // Loaded before the app so the window opens in the saved mode (`--windowed` forces a window).
    let settings = settings::load();
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Battle Tanks".into(),
                mode: settings.window_mode(),
                resolution: (1600, 900).into(),
                present_mode: settings.present_mode(),
                ..default()
            }),
            primary_cursor_options: Some(CursorOptions { visible: false, ..default() }),
            ..default()
        }))
        .insert_resource(settings)
        .add_plugins((settings::plugin, gfx::plugin, audio::plugin, input::plugin, game::plugin, ui::plugin, dev::plugin))
        .run();
}
