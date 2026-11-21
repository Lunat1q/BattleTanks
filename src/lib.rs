//! The whole game as a library. The Windows exe (`main.rs`) calls `run()`; another host (a different OS shell,
//! a wrapper crate) calls `run_with` with its own `Platform` and extra plugins.

// Bevy systems naturally take many params with long query types.
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

mod achieve;
mod audio;
#[cfg(test)]
mod city_brushes;
#[cfg(test)]
mod city_gen;
mod city_map;
mod dev;
mod game;
mod geom;
mod gfx;
mod input;
mod levels;
mod music;
mod net;
mod rng;
mod rules;
mod scores;
mod settings;
mod terrain;
mod ui;

pub use achieve::{Ach, LIST as ACHIEVEMENTS};
pub use gfx::art_badges::{PX as BADGE_PX, pixels as badge_pixels};
pub use gfx::art_promo::banner as paint_banner;
pub use gfx::canvas;
pub use gfx::icon::paint as paint_icon;
pub use input::TvRemote;

use bevy::prelude::*;
use bevy::window::CursorOptions;
use std::path::PathBuf;

/// What a host changes about the game. `Platform::default()` is the desktop exe.
#[derive(Resource, Clone, Debug)]
pub struct Platform {
    /// Folder for settings and City Run progress; `None` = `%APPDATA%\BattleTanks`.
    pub data_dir: Option<PathBuf>,
    /// HDR bloom on bullets and explosions; costly on TV-class GPUs.
    pub bloom: bool,
    /// Bevy's gilrs gamepad backend. Off when the host feeds `RawGamepadEvent`s itself.
    pub gilrs: bool,
    /// The host owns the window and swap chain: the settings screen hides FULLSCREEN and VSYNC.
    pub host_window: bool,
    /// Back on the title with nobody joined quits (TV launcher convention).
    pub back_exits_title: bool,
    /// Sends a finished classic run to the world board: `(url, game key, JSON) -> answer`, on a worker thread.
    /// Default: the OS `curl`. `None` = offline (local hi score only).
    pub post_score: Option<fn(&str, &str, &str) -> Result<String, String>>,
    /// Reads a world board for the HI SCORES screen: `url -> answer`, on a worker thread. Default: the OS `curl`.
    /// `None` = the screen says the board is not available.
    pub fetch_board: Option<fn(&str) -> Result<String, String>>,
    /// LAN CO-OP (two machines, one game): called with `true` when the LAN screen opens, answering this machine's
    /// name for the other side's list (a phone may also take its Wi-Fi multicast lock here), and `false` when it
    /// closes. `None` hides LAN CO-OP. Default: the computer name.
    pub lan: Option<fn(bool) -> String>,
    /// One player's cloud copy of City Run progress (the text of `city.txt`, "" = none yet), shared by their
    /// devices, e.g. Play Games saved games: `cloud_load() -> text` and `cloud_save(text)`, both on a worker thread.
    /// The game merges (unlocks only grow). `None` = this device only (default).
    pub cloud_load: Option<fn() -> Result<String, String>>,
    pub cloud_save: Option<fn(&str) -> Result<String, String>>,
    /// Achievements (`.claude/docs/achievements.md`), e.g. Play Games: `achieve(key, 0)` unlocks, `achieve(key, n)`
    /// sets a counter to `n` steps; on a worker thread, after each change and for everything once at start (both
    /// idempotent). `None` = no achievements at all: no tracking, no menu, no run summary (default).
    pub achieve: Option<fn(&str, u32)>,
}

/// The desktop's name for LAN CO-OP.
fn computer_name(_open: bool) -> String {
    std::env::var("COMPUTERNAME").or_else(|_| std::env::var("HOSTNAME")).unwrap_or_else(|_| "PC".into())
}

impl Default for Platform {
    fn default() -> Self {
        Platform {
            data_dir: None,
            bloom: true,
            gilrs: true,
            host_window: false,
            back_exits_title: false,
            post_score: Some(scores::curl_post),
            fetch_board: Some(scores::curl_get),
            lan: Some(computer_name),
            cloud_load: None,
            cloud_save: None,
            achieve: None,
        }
    }
}

pub fn run() {
    run_with(Platform::default(), |_| {});
}

/// Build and run the game for `platform`; `extra` adds the host's own plugins before the app starts.
pub fn run_with(platform: Platform, extra: impl FnOnce(&mut App)) {
    if let Some(dir) = &platform.data_dir {
        settings::set_data_dir(dir.clone());
    }
    // Loaded before the app so the window opens in the saved mode (`--windowed` forces a window).
    let settings = settings::load();
    let mut defaults = DefaultPlugins.build();
    if !platform.gilrs {
        defaults = defaults.disable::<bevy::gilrs::GilrsPlugin>();
    }
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
    .insert_resource(platform)
    .add_plugins((
        settings::plugin,
        gfx::plugin,
        audio::plugin,
        music::plugin,
        input::plugin,
        game::plugin,
        achieve::plugin,
        scores::plugin,
        ui::plugin,
        dev::plugin,
        net::plugin,
        dev::lan_plugin,
    ));
    extra(&mut app);
    app.run();
}
