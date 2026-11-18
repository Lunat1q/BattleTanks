//! Player settings, saved to `%APPDATA%\BattleTanks\settings.txt`: video (FPS limit, VSync, fullscreen),
//! volume, screen shake, FPS counter, friendly fire, controller vibration. Also owns the frame limiter, the F11 toggle and the FPS overlay.

use bevy::diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::text::FontSize;
use bevy::window::{MonitorSelection, PresentMode, WindowMode};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// FPS limit choices; 0 = unlimited.
pub const FPS_CHOICES: [u32; 8] = [30, 60, 90, 120, 144, 165, 240, 0];

#[derive(Resource, Clone, Copy, PartialEq, Debug)]
pub struct Settings {
    pub fps_limit: u32,
    pub vsync: bool,
    pub fullscreen: bool,
    /// 0..=100 percent, steps of 10.
    pub volume: u32,
    pub shake: bool,
    pub show_fps: bool,
    /// Players' bullets paralyze each other (classic). Off: they pass through.
    pub friendly_fire: bool,
    pub vibration: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            fps_limit: 60,
            vsync: true,
            fullscreen: true,
            volume: 80,
            shake: true,
            show_fps: false,
            friendly_fire: true,
            vibration: true,
        }
    }
}

/// Menu rows, in order. The settings screen adds BACK after these.
pub const ITEMS: [&str; 8] = ["FPS LIMIT", "VSYNC", "FULLSCREEN", "VOLUME", "SCREEN SHAKE", "SHOW FPS", "FRIENDLY FIRE", "VIBRATION"];

fn on_off(b: bool) -> &'static str {
    if b { "ON" } else { "OFF" }
}

impl Settings {
    /// `key=value` lines; unknown keys and bad values fall back to defaults.
    pub fn parse(text: &str) -> Self {
        let mut s = Settings::default();
        let flag = |v: &str, d: bool| match v {
            "true" => true,
            "false" => false,
            _ => d,
        };
        for (k, v) in text.lines().filter_map(|l| l.split_once('=')) {
            let v = v.trim();
            match k.trim() {
                "fps_limit" => s.fps_limit = v.parse().ok().filter(|f| FPS_CHOICES.contains(f)).unwrap_or(s.fps_limit),
                "vsync" => s.vsync = flag(v, s.vsync),
                "fullscreen" => s.fullscreen = flag(v, s.fullscreen),
                "volume" => s.volume = v.parse::<u32>().map(|n| n.min(100) / 10 * 10).unwrap_or(s.volume),
                "shake" => s.shake = flag(v, s.shake),
                "show_fps" => s.show_fps = flag(v, s.show_fps),
                "friendly_fire" => s.friendly_fire = flag(v, s.friendly_fire),
                "vibration" => s.vibration = flag(v, s.vibration),
                _ => {}
            }
        }
        s
    }

    pub fn serialize(&self) -> String {
        format!(
            "fps_limit={}\nvsync={}\nfullscreen={}\nvolume={}\nshake={}\nshow_fps={}\nfriendly_fire={}\nvibration={}\n",
            self.fps_limit, self.vsync, self.fullscreen, self.volume, self.shake, self.show_fps, self.friendly_fire, self.vibration
        )
    }

    /// Change row `item` of `ITEMS` one step (left = back, right/A = forward).
    pub fn adjust(&mut self, item: usize, forward: bool) {
        match item {
            0 => {
                let i = FPS_CHOICES.iter().position(|&f| f == self.fps_limit).unwrap_or(1);
                let n = FPS_CHOICES.len();
                self.fps_limit = FPS_CHOICES[if forward { (i + 1) % n } else { (i + n - 1) % n }];
            }
            1 => self.vsync = !self.vsync,
            2 => self.fullscreen = !self.fullscreen,
            3 => self.volume = if forward { (self.volume + 10).min(100) } else { self.volume.saturating_sub(10) },
            4 => self.shake = !self.shake,
            5 => self.show_fps = !self.show_fps,
            6 => self.friendly_fire = !self.friendly_fire,
            7 => self.vibration = !self.vibration,
            _ => {}
        }
    }

    /// Display value of row `item`.
    pub fn value(&self, item: usize) -> String {
        match item {
            0 if self.fps_limit == 0 => "OFF".into(),
            0 => self.fps_limit.to_string(),
            1 => on_off(self.vsync).into(),
            2 => on_off(self.fullscreen).into(),
            3 => format!("{}%", self.volume),
            4 => on_off(self.shake).into(),
            5 => on_off(self.show_fps).into(),
            6 => on_off(self.friendly_fire).into(),
            7 => on_off(self.vibration).into(),
            _ => String::new(),
        }
    }

    pub fn window_mode(&self) -> WindowMode {
        if self.fullscreen { WindowMode::BorderlessFullscreen(MonitorSelection::Current) } else { WindowMode::Windowed }
    }

    pub fn present_mode(&self) -> PresentMode {
        if self.vsync { PresentMode::AutoVsync } else { PresentMode::AutoNoVsync }
    }
}

fn path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("BattleTanks").join("settings.txt"))
}

/// Saved settings; `--windowed` on the command line forces windowed mode.
pub fn load() -> Settings {
    let mut s = path().and_then(|p| std::fs::read_to_string(p).ok()).map(|t| Settings::parse(&t)).unwrap_or_default();
    if std::env::args().any(|a| a == "--windowed") {
        s.fullscreen = false;
    }
    s
}

/// Best effort: a failed save only costs the setting on next launch.
pub fn save(s: &Settings) {
    let Some(p) = path() else { return };
    let result = p.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&p, s.serialize()));
    if let Err(e) = result {
        warn!("could not save settings to {}: {e}", p.display());
    }
}

#[derive(Component)]
struct FpsText;

/// Expects the `Settings` resource to be inserted already (main loads it to build the window).
pub fn plugin(app: &mut App) {
    app.add_plugins(FrameTimeDiagnosticsPlugin::default())
        .add_systems(Startup, spawn_fps_text)
        .add_systems(Update, (toggle_fullscreen, apply_window.run_if(resource_changed::<Settings>), fps_text).chain())
        .add_systems(Last, limit_fps);
}

fn apply_window(settings: Res<Settings>, mut windows: Query<&mut Window>) {
    for mut w in &mut windows {
        if w.present_mode != settings.present_mode() {
            w.present_mode = settings.present_mode();
        }
        if w.mode != settings.window_mode() {
            w.mode = settings.window_mode();
        }
    }
}

/// F11 / Alt+Enter flips the fullscreen setting (and saves it).
fn toggle_fullscreen(keys: Res<ButtonInput<KeyCode>>, mut settings: ResMut<Settings>) {
    if keys.just_pressed(KeyCode::F11) || (keys.pressed(KeyCode::AltLeft) && keys.just_pressed(KeyCode::Enter)) {
        settings.fullscreen = !settings.fullscreen;
        save(&settings);
    }
}

fn spawn_fps_text(mut commands: Commands) {
    commands.spawn((
        Text::new(""),
        TextFont { font_size: FontSize::Vh(2.4), ..default() },
        TextColor(Color::srgb(0.6, 0.9, 0.6)),
        Node { position_type: PositionType::Absolute, top: Val::Vh(1.0), right: Val::Vh(1.5), ..default() },
        GlobalZIndex(100),
        FpsText,
    ));
}

fn fps_text(settings: Res<Settings>, store: Res<DiagnosticsStore>, mut q: Query<(&mut Text, &mut Visibility), With<FpsText>>) {
    let Ok((mut text, mut vis)) = q.single_mut() else { return };
    vis.set_if_neq(if settings.show_fps { Visibility::Inherited } else { Visibility::Hidden });
    if let Some(fps) = settings.show_fps.then(|| store.get(&FrameTimeDiagnosticsPlugin::FPS).and_then(|d| d.smoothed())).flatten() {
        let s = format!("{fps:.0} FPS");
        if text.0 != s {
            text.0 = s;
        }
    }
}

/// Sleep at the end of each frame so frames are at least `1 / fps_limit` apart.
fn limit_fps(settings: Res<Settings>, mut last: Local<Option<Instant>>) {
    if settings.fps_limit > 0
        && let Some(prev) = *last
    {
        let end = prev + Duration::from_secs_f64(1.0 / settings.fps_limit as f64);
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
        let s = Settings {
            fps_limit: 144,
            vsync: false,
            fullscreen: false,
            volume: 30,
            shake: false,
            show_fps: true,
            friendly_fire: false,
            vibration: false,
        };
        assert_eq!(Settings::parse(&s.serialize()), s);
        assert_eq!(Settings::parse("fps_limit=77\nvsync=maybe\nvolume=loud\nnoise"), Settings::default());
        assert_eq!(Settings::parse("volume=250").volume, 100);
        assert_eq!(Settings::parse("volume=47").volume, 40);
        assert_eq!(Settings::parse(""), Settings::default());
    }

    #[test]
    fn fps_cycles_and_wraps() {
        let mut s = Settings { fps_limit: 240, ..default() };
        s.adjust(0, true);
        assert_eq!(s.value(0), "OFF");
        s.adjust(0, true);
        assert_eq!(s.fps_limit, 30);
        s.adjust(0, false);
        assert_eq!(s.fps_limit, 0);
    }

    #[test]
    fn volume_clamps_and_toggles_flip() {
        let mut s = Settings { volume: 100, ..default() };
        s.adjust(3, true);
        assert_eq!(s.volume, 100);
        for _ in 0..12 {
            s.adjust(3, false);
        }
        assert_eq!(s.value(3), "0%");
        for item in [1, 2, 4, 5, 6, 7] {
            let before = s.value(item);
            s.adjust(item, true);
            assert_ne!(s.value(item), before, "item {item}");
        }
    }
}
