//! The whole game as a library, so the Windows exe (`main.rs`) and the Android `cdylib` (`android_main` below)
//! run the same code. Android builds it with `cargo rustc --lib --crate-type cdylib` (ADR 0006).

// Bevy systems naturally take many params with long query types.
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

#[cfg(any(target_os = "android", test))]
mod android_pads;
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

pub fn run() {
    // Loaded before the app so the window opens in the saved mode (`--windowed` forces a window).
    let settings = settings::load();
    let defaults = DefaultPlugins.build();
    // gilrs has no Android backend; `android_pads` feeds Bevy's gamepads from Java instead (ADR 0006).
    #[cfg(target_os = "android")]
    let defaults = defaults.disable::<bevy::gilrs::GilrsPlugin>();
    let mut app = App::new();
    app.add_plugins(defaults.set(WindowPlugin {
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
    .add_plugins((settings::plugin, gfx::plugin, audio::plugin, input::plugin, game::plugin, ui::plugin, dev::plugin));
    #[cfg(target_os = "android")]
    app.add_plugins(android_pads::plugin);
    app.run();
}

/// Android entry point: `#[bevy_main]` stores the `AndroidApp` in `bevy::android::ANDROID_APP`, then calls this.
#[cfg(target_os = "android")]
#[bevy_main]
fn main() {
    run();
}
