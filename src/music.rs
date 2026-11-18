//! Background music: the owner's own tracks (`assets/music/*.ogg`), baked into the exe like the stage maps.
//! One track plays at a time from a pool picked by the screen, stage and (City Run) whether a boss is up; when
//! it ends the next track of the pool starts, and a new pick switches at once. The pause menu pauses it.
//! All tracks are normalised to -16 LUFS when encoded (see domain.md, Operational lessons).

use crate::game::{Enemy, Mode, Paused, Screen, Session};
use crate::rules::Rank;
use crate::settings::Settings;
use bevy::audio::{AudioPlayer, AudioSink, AudioSinkPlayback, AudioSource, PlaybackSettings, Volume};
use bevy::prelude::*;
use bevy::window::AppLifecycle;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Track {
    Menu,
    MenuAlt,
    Slow1,
    Slow2,
    Medium1,
    Medium2,
    Fast1,
    Fast2,
    Fast3,
    Fast4,
    HardFight1,
    HardFight2,
    BossFight1,
    BossFight2,
    Victory,
    Loss,
}

use Track::*;

const ALL: [Track; 16] = [
    Menu, MenuAlt, Slow1, Slow2, Medium1, Medium2, Fast1, Fast2, Fast3, Fast4, HardFight1, HardFight2, BossFight1, BossFight2, Victory,
    Loss,
];

// Pools a screen rotates through: when a track ends the next one in its pool starts.
const SLOW: &[Track] = &[Slow1, Slow2];
const MEDIUM: &[Track] = &[Medium1, Medium2];
const FAST: &[Track] = &[Fast1, Fast2, Fast3, Fast4];
/// City districts are long, so their pools also reach one speed up.
const CITY_SLOW: &[Track] = &[Slow1, Slow2, Medium1, Medium2];
const CITY_MEDIUM: &[Track] = &[Medium1, Medium2, Fast1, Fast2];
const HARD: &[Track] = &[HardFight1, HardFight2];
const BOSS: &[Track] = &[BossFight1, BossFight2];

fn bytes(t: Track) -> &'static [u8] {
    match t {
        Menu => include_bytes!("../assets/music/menu_music.ogg"),
        MenuAlt => include_bytes!("../assets/music/menu_music_alternative.ogg"),
        Slow1 => include_bytes!("../assets/music/battle_music_slow_1.ogg"),
        Slow2 => include_bytes!("../assets/music/battle_music_slow_2.ogg"),
        Medium1 => include_bytes!("../assets/music/battle_music_medium_1.ogg"),
        Medium2 => include_bytes!("../assets/music/battle_music_medium_2.ogg"),
        Fast1 => include_bytes!("../assets/music/battle_music_fast_1.ogg"),
        Fast2 => include_bytes!("../assets/music/battle_music_fast_2.ogg"),
        Fast3 => include_bytes!("../assets/music/battle_music_fast_3.ogg"),
        Fast4 => include_bytes!("../assets/music/battle_music_fast_4.ogg"),
        HardFight1 => include_bytes!("../assets/music/battle_hardfight_music_fast_1.ogg"),
        HardFight2 => include_bytes!("../assets/music/battle_hardfight_music_fast_2.ogg"),
        BossFight1 => include_bytes!("../assets/music/battle_bossfight_music_fast_1.ogg"),
        BossFight2 => include_bytes!("../assets/music/battle_bossfight_music_fast_2.ogg"),
        Victory => include_bytes!("../assets/music/battle_music_victory_1.ogg"),
        Loss => include_bytes!("../assets/music/battle_music_loss_1.ogg"),
    }
}

/// Which boss, if any, is on the field (City Run).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fight {
    None,
    Mini,
    Boss,
}

/// Battle music gets faster as the game does: classic stages 1-10 slow, 11-20 medium, 21-30 fast (maps loop
/// after 30); City Run districts 1-3 slow, 4-7 medium, 8-10 fast. Returns the pool to rotate through.
pub fn pick(screen: Screen, mode: Mode, stage: usize, fight: Fight) -> &'static [Track] {
    let n = stage.max(1) - 1;
    match screen {
        Screen::Title | Screen::Settings => &[Menu],
        Screen::CityMap => &[MenuAlt],
        Screen::Results => &[Victory],
        Screen::GameOver => &[Loss],
        Screen::Stage => match (fight, mode) {
            (Fight::Boss, _) => BOSS,
            (Fight::Mini, _) => HARD,
            (_, Mode::Classic) => [SLOW, MEDIUM, FAST][n % 30 / 10],
            (_, Mode::City) => match n {
                0..=2 => CITY_SLOW,
                3..=6 => CITY_MEDIUM,
                _ => FAST,
            },
        },
    }
}

/// Track to play from `pool`: the one after `ended` if it was from this pool, else entry `start` (the stage
/// number, so consecutive stages open with different tracks).
pub fn next(pool: &[Track], ended: Option<Track>, start: usize) -> Track {
    let i = ended.and_then(|e| pool.iter().position(|&t| t == e)).map_or(start, |i| i + 1);
    pool[i % pool.len()]
}

#[derive(Resource)]
struct Music {
    handles: Vec<Handle<AudioSource>>,
    /// Pool, track, its player entity.
    playing: Option<(&'static [Track], Track, Entity)>,
}

pub fn plugin(app: &mut App) {
    // Same mute rule as the sound effects (smoke runs, `BT_MUTE=1`).
    if std::env::var_os("BT_SHOT").is_some() || std::env::var_os("BT_MUTE").is_some() {
        return;
    }
    app.add_systems(Startup, load).add_systems(PostUpdate, (switch, mix).chain());
}

fn load(mut commands: Commands, mut sources: ResMut<Assets<AudioSource>>) {
    let handles = ALL.iter().map(|&t| sources.add(AudioSource { bytes: bytes(t).into() })).collect();
    commands.insert_resource(Music { handles, playing: None });
}

fn switch(
    mut commands: Commands,
    mut music: ResMut<Music>,
    screen: Res<State<Screen>>,
    session: Res<Session>,
    settings: Res<Settings>,
    enemies: Query<&Enemy>,
) {
    let fight = enemies.iter().fold(Fight::None, |f, e| match e.rank {
        Rank::Boss => Fight::Boss,
        Rank::Mini if f == Fight::None => Fight::Mini,
        _ => f,
    });
    let want = (settings.music > 0).then(|| pick(*screen.get(), session.mode, session.stage, fight));
    // A finished track despawns itself (DESPAWN, not LOOP: looping buffers the whole decoded song in RAM).
    let alive = music.playing.is_some_and(|(.., e)| commands.get_entity(e).is_ok());
    let same_pool = music.playing.map(|(p, ..)| p) == want;
    if alive && same_pool {
        return;
    }
    let ended = music.playing.take().and_then(|(_, t, e)| {
        if let Ok(mut ec) = commands.get_entity(e) {
            ec.despawn();
        }
        same_pool.then_some(t)
    });
    if let Some(pool) = want {
        let t = next(pool, ended, session.stage.max(1) - 1);
        let i = ALL.iter().position(|&a| a == t).unwrap_or(0);
        let volume = Volume::Linear(settings.music as f32 / 100.0);
        let e = commands.spawn((AudioPlayer(music.handles[i].clone()), PlaybackSettings::DESPAWN.with_volume(volume))).id();
        music.playing = Some((pool, t, e));
    }
}

/// Follow the MUSIC setting live and hold the track while the pause menu is up or the app is in the background
/// (the audio thread keeps playing when the OS suspends the app).
fn mix(
    mut lifecycle: MessageReader<AppLifecycle>,
    mut away: Local<bool>,
    settings: Res<Settings>,
    paused: Option<Res<Paused>>,
    screen: Res<State<Screen>>,
    mut sinks: Query<&mut AudioSink>,
    music: Res<Music>,
) {
    if let Some(b) = lifecycle.read().filter_map(|&e| crate::game::stage::backgrounded(e)).last() {
        *away = b;
    }
    let Some((.., e)) = music.playing else { return };
    let Ok(mut sink) = sinks.get_mut(e) else { return };
    let volume = Volume::Linear(settings.music as f32 / 100.0);
    if sink.volume() != volume {
        sink.set_volume(volume);
    }
    let hold = *away || (*screen.get() == Screen::Stage && paused.is_some_and(|p| p.0));
    if hold != sink.is_paused() {
        if hold { sink.pause() } else { sink.play() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_follow_the_screen_and_speed_up() {
        let first = |mode, n| next(pick(Screen::Stage, mode, n, Fight::None), None, n - 1);
        assert_eq!(pick(Screen::Settings, Mode::Classic, 1, Fight::None), [Menu]);
        assert_eq!(pick(Screen::CityMap, Mode::City, 1, Fight::None), [MenuAlt]);
        assert_eq!(pick(Screen::Results, Mode::City, 5, Fight::Boss), [Victory]);
        assert_eq!(pick(Screen::GameOver, Mode::Classic, 5, Fight::None), [Loss]);
        assert_eq!(
            [1, 2, 10, 11, 12, 20, 21, 30, 31].map(|n| first(Mode::Classic, n)),
            [Slow1, Slow2, Slow2, Medium1, Medium2, Medium2, Fast1, Fast2, Slow1]
        );
        assert_eq!(
            (1..=10).map(|d| first(Mode::City, d)).collect::<Vec<_>>(),
            [Slow1, Slow2, Medium1, Fast2, Medium1, Medium2, Fast1, Fast4, Fast1, Fast2]
        );
        assert_eq!(pick(Screen::Stage, Mode::City, 2, Fight::Mini), HARD);
        assert_eq!(pick(Screen::Stage, Mode::City, 9, Fight::Boss), BOSS);
    }

    #[test]
    fn an_ended_track_hands_over_to_the_next_in_its_pool() {
        assert_eq!(next(FAST, Some(Fast2), 0), Fast3);
        assert_eq!(next(FAST, Some(Fast4), 0), Fast1, "wraps");
        assert_eq!(next(BOSS, Some(Fast4), 1), BossFight2, "track from another pool: start fresh");
        assert_eq!(next(&[Menu], Some(Menu), 7), Menu, "single-track pool repeats");
        let mut t = Slow1;
        for _ in 0..4 {
            let n = next(CITY_SLOW, Some(t), 0);
            assert_ne!(n, t, "never the same track twice in a row");
            t = n;
        }
    }

    #[test]
    fn every_track_decodes() {
        use bevy::audio::{Decodable, Source};
        for t in ALL {
            let mut d = AudioSource { bytes: bytes(t).into() }.decoder();
            assert_eq!((d.sample_rate().get(), d.channels().get()), (44_100, 2), "{t:?}");
            assert!(d.by_ref().take(44_100).any(|x| x != 0.0), "{t:?} is silent");
        }
    }
}
