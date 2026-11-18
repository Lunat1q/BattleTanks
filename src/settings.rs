//! Player settings (FPS limit, VSync), saved to `%APPDATA%\BattleTanks\settings.txt`, plus the frame limiter.

use bevy::prelude::*;
use bevy::window::PresentMode;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// FPS limit choices; 0 = unlimited.
pub const FPS_CHOICES: [u32; 8] = [30, 60, 90, 120, 144, 165, 240, 0];

#[derive(Resource, Clone, Copy, PartialEq, Debug)]
pub struct Settings {
    pub fps_limit: u32,
    pub vsync: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { fps_limit: 60, vsync: true }
    }
}

impl Settings {
    /// `key=value` lines; unknown keys and bad values fall back to defaults.
    pub fn parse(text: &str) -> Self {
        let mut s = Settings::default();
        for (k, v) in text.lines().filter_map(|l| l.split_once('=')) {
            match (k.trim(), v.trim()) {
                ("fps_limit", v) => s.fps_limit = v.parse().ok().filter(|f| FPS_CHOICES.contains(f)).unwrap_or(s.fps_limit),
                ("vsync", v) => s.vsync = v != "false",
                _ => {}
            }
        }
        s
    }

    pub fn serialize(&self) -> String {
        format!("fps_limit={}\nvsync={}\n", self.fps_limit, self.vsync)
    }

    /// Step the FPS limit through `FPS_CHOICES` (wraps).
    pub fn cycle_fps(&mut self, forward: bool) {
        let i = FPS_CHOICES.iter().position(|&f| f == self.fps_limit).unwrap_or(1);
        let n = FPS_CHOICES.len();
        self.fps_limit = FPS_CHOICES[if forward { (i + 1) % n } else { (i + n - 1) % n }];
    }
}

fn path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("BattleTanks").join("settings.txt"))
}

fn load() -> Settings {
    path().and_then(|p| std::fs::read_to_string(p).ok()).map(|t| Settings::parse(&t)).unwrap_or_default()
}

/// Best effort: a failed save only costs the setting on next launch.
pub fn save(s: &Settings) {
    let Some(p) = path() else { return };
    let result = p.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&p, s.serialize()));
    if let Err(e) = result {
        warn!("could not save settings to {}: {e}", p.display());
    }
}

pub fn plugin(app: &mut App) {
    app.insert_resource(load()).add_systems(Update, apply_vsync.run_if(resource_changed::<Settings>)).add_systems(Last, limit_fps);
}

fn apply_vsync(settings: Res<Settings>, mut windows: Query<&mut Window>) {
    let mode = if settings.vsync { PresentMode::AutoVsync } else { PresentMode::AutoNoVsync };
    for mut w in &mut windows {
        if w.present_mode != mode {
            w.present_mode = mode;
        }
    }
}

/// Sleep at the end of each frame so frames are at least `1 / fps_limit` apart.
fn limit_fps(settings: Res<Settings>, mut last: Local<Option<Instant>>) {
    if settings.fps_limit > 0
        && let Some(prev) = *last
    {
        let target = Duration::from_secs_f64(1.0 / settings.fps_limit as f64);
        let end = prev + target;
        // OS sleep is ~1 ms coarse: sleep most of the gap, spin the rest for even frame pacing.
        if let Some(coarse) = end.checked_duration_since(Instant::now()).and_then(|d| d.checked_sub(Duration::from_micros(1500))) {
            std::thread::sleep(coarse);
        }
        while Instant::now() < end {
            std::hint::spin_loop();
        }
    }
    *last = Some(Instant::now());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_rejects_junk() {
        let s = Settings { fps_limit: 144, vsync: false };
        assert_eq!(Settings::parse(&s.serialize()), s);
        assert_eq!(Settings::parse("fps_limit=77\nvsync=maybe\nnoise"), Settings { fps_limit: 60, vsync: true });
        assert_eq!(Settings::parse(""), Settings::default());
    }

    #[test]
    fn fps_cycles_and_wraps() {
        let mut s = Settings { fps_limit: 240, vsync: true };
        s.cycle_fps(true);
        assert_eq!(s.fps_limit, 0, "unlimited after the highest");
        s.cycle_fps(true);
        assert_eq!(s.fps_limit, 30);
        s.cycle_fps(false);
        assert_eq!(s.fps_limit, 0);
    }
}
