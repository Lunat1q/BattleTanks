//! Background music: the owner's own tracks (`assets/music/*.ogg`), baked into the exe like the stage maps.
//! One track plays at a time, picked from the screen, stage and (City Run) whether a boss is up; it restarts
//! when it ends and switches when the pick changes. The pause menu pauses it.

use crate::game::{Enemy, Mode, Paused, Screen, Session};
use crate::rules::Rank;
use crate::settings::Settings;
use bevy::audio::{AudioPlayer, AudioSink, AudioSinkPlayback, AudioSource, PlaybackSettings, Volume};
use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Track {
    Menu,
    MenuAlt,
    Slow1,
    Slow2,
    Medium1,
    Medium2,
    Fast,
    HardFight,
    BossFight,
    Victory,
    Loss,
}

const ALL: [Track; 11] = [
    Track::Menu,
    Track::MenuAlt,
    Track::Slow1,
    Track::Slow2,
    Track::Medium1,
    Track::Medium2,
    Track::Fast,
    Track::HardFight,
    Track::BossFight,
    Track::Victory,
    Track::Loss,
];

fn bytes(t: Track) -> &'static [u8] {
    match t {
        Track::Menu => include_bytes!("../assets/music/menu_music.ogg"),
        Track::MenuAlt => include_bytes!("../assets/music/menu_music_alternative.ogg"),
        Track::Slow1 => include_bytes!("../assets/music/battle_music_slow_1.ogg"),
        Track::Slow2 => include_bytes!("../assets/music/battle_music_slow_2.ogg"),
        Track::Medium1 => include_bytes!("../assets/music/battle_music_medium_1.ogg"),
        Track::Medium2 => include_bytes!("../assets/music/battle_music_medium_2.ogg"),
        Track::Fast => include_bytes!("../assets/music/battle_music_fast_1.ogg"),
        Track::HardFight => include_bytes!("../assets/music/battle_hardfight_music_fast_1.ogg"),
        Track::BossFight => include_bytes!("../assets/music/battle_bossfight_music_fast_1.ogg"),
        Track::Victory => include_bytes!("../assets/music/battle_music_victory_1.ogg"),
        Track::Loss => include_bytes!("../assets/music/battle_music_loss_1.ogg"),
    }
}

/// Which boss, if any, is on the field (City Run).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Fight {
    None,
    Mini,
    Boss,
}

/// Battle tracks get faster as the game does: classic stages 1-10 slow, 11-20 medium, 21-30 fast (maps loop
/// after 30); City Run districts 1-3 slow, 4-7 medium, 8-10 fast. Odd/even alternates the two variants.
pub fn pick(screen: Screen, mode: Mode, stage: usize, fight: Fight) -> Track {
    let n = stage.max(1) - 1;
    let tier = match mode {
        Mode::Classic => n % 30 / 10,
        Mode::City => match n {
            0..=2 => 0,
            3..=6 => 1,
            _ => 2,
        },
    };
    match screen {
        Screen::Title | Screen::Settings => Track::Menu,
        Screen::CityMap => Track::MenuAlt,
        Screen::Results => Track::Victory,
        Screen::GameOver => Track::Loss,
        Screen::Stage => match (fight, tier, n % 2) {
            (Fight::Boss, ..) => Track::BossFight,
            (Fight::Mini, ..) => Track::HardFight,
            (_, 0, 0) => Track::Slow1,
            (_, 0, _) => Track::Slow2,
            (_, 1, 0) => Track::Medium1,
            (_, 1, _) => Track::Medium2,
            _ => Track::Fast,
        },
    }
}

#[derive(Resource)]
struct Music {
    handles: Vec<Handle<AudioSource>>,
    playing: Option<(Track, Entity)>,
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
    let alive = music.playing.is_some_and(|(_, e)| commands.get_entity(e).is_ok());
    if alive && music.playing.map(|(t, _)| t) == want {
        return;
    }
    if let Some((_, e)) = music.playing.take()
        && let Ok(mut ec) = commands.get_entity(e)
    {
        ec.despawn();
    }
    if let Some(t) = want {
        let i = ALL.iter().position(|&a| a == t).unwrap_or(0);
        let volume = Volume::Linear(settings.music as f32 / 100.0);
        let e = commands.spawn((AudioPlayer(music.handles[i].clone()), PlaybackSettings::DESPAWN.with_volume(volume))).id();
        music.playing = Some((t, e));
    }
}

/// Follow the MUSIC setting live and hold the track while the pause menu is up.
fn mix(
    settings: Res<Settings>,
    paused: Option<Res<Paused>>,
    screen: Res<State<Screen>>,
    mut sinks: Query<&mut AudioSink>,
    music: Res<Music>,
) {
    let Some((_, e)) = music.playing else { return };
    let Ok(mut sink) = sinks.get_mut(e) else { return };
    let volume = Volume::Linear(settings.music as f32 / 100.0);
    if sink.volume() != volume {
        sink.set_volume(volume);
    }
    let hold = *screen.get() == Screen::Stage && paused.is_some_and(|p| p.0);
    if hold != sink.is_paused() {
        if hold { sink.pause() } else { sink.play() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_follow_the_screen_and_speed_up() {
        let stage = |mode, n| pick(Screen::Stage, mode, n, Fight::None);
        assert_eq!(pick(Screen::Settings, Mode::Classic, 1, Fight::None), Track::Menu);
        assert_eq!(pick(Screen::CityMap, Mode::City, 1, Fight::None), Track::MenuAlt);
        assert_eq!(pick(Screen::Results, Mode::City, 5, Fight::Boss), Track::Victory);
        assert_eq!(pick(Screen::GameOver, Mode::Classic, 5, Fight::None), Track::Loss);
        assert_eq!([1, 2, 10, 11, 12, 20, 21, 30, 31].map(|n| stage(Mode::Classic, n)), {
            use Track::*;
            [Slow1, Slow2, Slow2, Medium1, Medium2, Medium2, Fast, Fast, Slow1]
        });
        assert_eq!((1..=10).map(|d| stage(Mode::City, d)).collect::<Vec<_>>(), {
            use Track::*;
            vec![Slow1, Slow2, Slow1, Medium2, Medium1, Medium2, Medium1, Fast, Fast, Fast]
        });
        assert_eq!(pick(Screen::Stage, Mode::City, 2, Fight::Mini), Track::HardFight);
        assert_eq!(pick(Screen::Stage, Mode::City, 9, Fight::Boss), Track::BossFight);
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
