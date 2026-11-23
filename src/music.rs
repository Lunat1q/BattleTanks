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

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Track {
    Menu,
    MenuAlt,
    Slow1,
    Slow2,
    Slow3,
    Slow4,
    Slow5,
    Upbeat1,
    Upbeat2,
    Medium1,
    Medium2,
    Medium3,
    Medium4,
    Medium5,
    Fast1,
    Fast2,
    Fast3,
    Fast4,
    Fast5,
    Fast6,
    HardFight1,
    HardFight2,
    HardFight3,
    HardFight4,
    HardFight5,
    BossFight1,
    BossFight2,
    BossFight3,
    BossFight4,
    BossFight5,
    Victory,
    Loss1,
    Loss2,
}

use Track::*;

pub static ALL: [Track; 33] = [
    Menu, MenuAlt, Slow1, Slow2, Slow3, Slow4, Slow5, Upbeat1, Upbeat2, Medium1, Medium2, Medium3, Medium4, Medium5, Fast1, Fast2, Fast3,
    Fast4, Fast5, Fast6, HardFight1, HardFight2, HardFight3, HardFight4, HardFight5, BossFight1, BossFight2, BossFight3, BossFight4,
    BossFight5, Victory, Loss1, Loss2,
];

/// OST names for the bunker's jukebox, in `ALL` order: (name, where it plays). Ours to rename.
pub const NAMES: [(&str, &str); 33] = [
    ("ROLL CALL", "title"),
    ("CITY LIGHTS", "district map"),
    ("MORNING PATROL", "stages 1-10"),
    ("BRICK BY BRICK", "stages 1-10"),
    ("DUG IN", "stages 1-10"),
    ("FOG OF WAR", "stages 1-10"),
    ("NIGHT WATCH", "stages 1-10"),
    ("TREAD LIGHTLY", "stages 1-10"),
    ("SUNDAY DRIVE", "stages 1-10"),
    ("ADVANCE", "stages 11-20"),
    ("CROSSFIRE", "stages 11-20"),
    ("STEEL RAIN", "stages 11-20"),
    ("FLANK LEFT", "stages 11-20"),
    ("HOLD THE LINE", "stages 11-20"),
    ("FULL THROTTLE", "stages 21-30"),
    ("OVERHEAT", "stages 21-30"),
    ("BREAKTHROUGH", "stages 21-30"),
    ("BLITZ", "stages 21-30"),
    ("NO BRAKES", "stages 21-30"),
    ("RED LINE", "stages 21-30"),
    ("HEAVY METAL", "mini-boss"),
    ("ARMOUR PIERCING", "mini-boss"),
    ("IRON FIST", "mini-boss"),
    ("SIEGE ENGINE", "mini-boss"),
    ("WAR MACHINE", "mini-boss"),
    ("THE BIG ONE", "district boss"),
    ("LAST BASTION", "district boss"),
    ("FINAL STAND", "district boss"),
    ("TITAN", "district boss"),
    ("CROWN OF STEEL", "district boss"),
    ("STAGE CLEAR", "results"),
    ("BASE DOWN", "game over"),
    ("SCRAP HEAP", "game over"),
];

/// The bunker's jukebox pick (`ALL` index): plays instead of the screen's pool while the bunker is open, then the
/// next track of the OST when it ends. The second field holds it where it is (A on the playing track again).
#[derive(Resource, Default)]
pub struct Jukebox(pub Option<usize>, pub bool);

impl Jukebox {
    /// A on track `i`: the playing one pauses or resumes, any other starts playing.
    pub fn press(&mut self, i: usize) {
        *self = Jukebox(Some(i), self.0 == Some(i) && !self.1);
    }
}

// Pools a screen plays from: a stage opens with the track its number picks, then shuffles (`next`).
const SLOW: &[Track] = &[Slow1, Slow2, Slow3, Slow4, Slow5, Upbeat1, Upbeat2];
const MEDIUM: &[Track] = &[Medium1, Medium2, Medium3, Medium4, Medium5];
const FAST: &[Track] = &[Fast1, Fast2, Fast3, Fast4, Fast5, Fast6];
/// City districts are long, so their pools also reach one speed up.
const CITY_SLOW: &[Track] = &[Slow1, Slow2, Slow3, Slow4, Medium1, Medium2, Slow5, Upbeat1, Upbeat2];
const CITY_MEDIUM: &[Track] = &[Medium1, Medium2, Medium3, Medium4, Medium5, Fast1, Fast2];
const LOSS: &[Track] = &[Loss1, Loss2];
const HARD: &[Track] = &[HardFight1, HardFight2, HardFight3, HardFight4, HardFight5];
const BOSS: &[Track] = &[BossFight1, BossFight2, BossFight3, BossFight4, BossFight5];

fn bytes(t: Track) -> &'static [u8] {
    match t {
        Menu => include_bytes!("../assets/music/menu_music.ogg"),
        MenuAlt => include_bytes!("../assets/music/menu_music_alternative.ogg"),
        Slow1 => include_bytes!("../assets/music/battle_music_slow_1.ogg"),
        Slow2 => include_bytes!("../assets/music/battle_music_slow_2.ogg"),
        Slow3 => include_bytes!("../assets/music/battle_music_slow_3.ogg"),
        Slow4 => include_bytes!("../assets/music/battle_music_slow_4.ogg"),
        Slow5 => include_bytes!("../assets/music/battle_music_slow_5.ogg"),
        Upbeat1 => include_bytes!("../assets/music/battle_upbeat_music_slow_1.ogg"),
        Upbeat2 => include_bytes!("../assets/music/battle_upbeat_music_slow_2.ogg"),
        Medium1 => include_bytes!("../assets/music/battle_music_medium_1.ogg"),
        Medium2 => include_bytes!("../assets/music/battle_music_medium_2.ogg"),
        Medium3 => include_bytes!("../assets/music/battle_music_medium_3.ogg"),
        Medium4 => include_bytes!("../assets/music/battle_music_medium_4.ogg"),
        Medium5 => include_bytes!("../assets/music/battle_music_medium_5.ogg"),
        Fast1 => include_bytes!("../assets/music/battle_music_fast_1.ogg"),
        Fast2 => include_bytes!("../assets/music/battle_music_fast_2.ogg"),
        Fast3 => include_bytes!("../assets/music/battle_music_fast_3.ogg"),
        Fast4 => include_bytes!("../assets/music/battle_music_fast_4.ogg"),
        Fast5 => include_bytes!("../assets/music/battle_music_fast_5.ogg"),
        Fast6 => include_bytes!("../assets/music/battle_music_fast_6.ogg"),
        HardFight1 => include_bytes!("../assets/music/battle_hardfight_music_fast_1.ogg"),
        HardFight2 => include_bytes!("../assets/music/battle_hardfight_music_fast_2.ogg"),
        HardFight3 => include_bytes!("../assets/music/battle_hardfight_music_fast_3.ogg"),
        HardFight4 => include_bytes!("../assets/music/battle_hardfight_music_fast_4.ogg"),
        HardFight5 => include_bytes!("../assets/music/battle_hardfight_music_fast_5.ogg"),
        BossFight1 => include_bytes!("../assets/music/battle_bossfight_music_fast_1.ogg"),
        BossFight2 => include_bytes!("../assets/music/battle_bossfight_music_fast_2.ogg"),
        BossFight3 => include_bytes!("../assets/music/battle_bossfight_music_fast_3.ogg"),
        BossFight4 => include_bytes!("../assets/music/battle_bossfight_music_fast_4.ogg"),
        BossFight5 => include_bytes!("../assets/music/battle_bossfight_music_fast_5.ogg"),
        Victory => include_bytes!("../assets/music/battle_music_victory_1.ogg"),
        Loss1 => include_bytes!("../assets/music/battle_music_loss_1.ogg"),
        Loss2 => include_bytes!("../assets/music/battle_music_loss_2.ogg"),
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
        Screen::Title | Screen::StageSelect | Screen::Settings | Screen::HiScores | Screen::Lan | Screen::Achievements | Screen::Bunker => {
            &[Menu]
        }
        Screen::CityMap => &[MenuAlt],
        Screen::Results | Screen::Unlocks => &[Victory],
        Screen::GameOver => LOSS,
        // Keeps the screen before it playing: a district clear's victory, or a classic run's game over.
        Screen::Board if mode == Mode::City => &[Victory],
        Screen::Board => LOSS,
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

/// Track to play from `pool`. A stage pool opens with entry `start` (the stage number, so consecutive stages open
/// with different tracks); mini-boss and boss fights (`start` None) open on a random one. After that it shuffles:
/// any track of the pool but the one that just `ended`. `roll` is a random number (music's own dice, not the
/// gameplay ones: LAN peers may hear different tracks).
pub fn next(pool: &[Track], ended: Option<Track>, start: Option<usize>, roll: u32) -> Track {
    let n = pool.len();
    match (ended.and_then(|e| pool.iter().position(|&t| t == e)), start) {
        (Some(i), _) if n > 1 => pool[(i + 1 + roll as usize % (n - 1)) % n],
        (None, Some(s)) => pool[s % n],
        _ => pool[roll as usize % n],
    }
}

/// The track a new pick goes on from: the one that just ended (same pool), else the track the old pool itself
/// cut in on, kept in `before`. So when a mini-boss or boss dies the stage music moves on to the track after the
/// one the fight interrupted instead of restarting its pool.
fn handover(same_pool: bool, old: Option<Track>, before: &mut Option<Track>) -> Option<Track> {
    if same_pool { old } else { std::mem::replace(before, old) }
}

#[derive(Resource)]
struct Music {
    handles: Vec<Handle<AudioSource>>,
    /// Pool, track, its player entity.
    playing: Option<(&'static [Track], Track, Entity)>,
    /// Track the playing pool cut in on (`handover`); cleared off the stage, so each stage opens by its number.
    before: Option<Track>,
    rng: crate::rng::Rng,
}

pub fn plugin(app: &mut App) {
    // Same mute rule as the sound effects (smoke runs, `BT_MUTE=1`); the bunker still needs its pick.
    app.init_resource::<Jukebox>();
    if std::env::var_os("BT_SHOT").is_some() || std::env::var_os("BT_MUTE").is_some() {
        return;
    }
    app.add_systems(Startup, load).add_systems(PostUpdate, (switch, mix).chain());
}

fn load(mut commands: Commands, mut sources: ResMut<Assets<AudioSource>>) {
    let handles = ALL.iter().map(|&t| sources.add(AudioSource { bytes: bytes(t).into() })).collect();
    commands.insert_resource(Music { handles, playing: None, before: None, rng: crate::rng::Rng::from_time() });
}

fn switch(
    mut commands: Commands,
    mut music: ResMut<Music>,
    screen: Res<State<Screen>>,
    session: Res<Session>,
    settings: Res<Settings>,
    enemies: Query<&Enemy>,
    mut jukebox: ResMut<Jukebox>,
) {
    let fight = enemies.iter().fold(Fight::None, |f, e| match e.rank {
        Rank::Boss => Fight::Boss,
        Rank::Mini if f == Fight::None => Fight::Mini,
        _ => f,
    });
    // A finished track despawns itself (DESPAWN, not LOOP: looping buffers the whole decoded song in RAM).
    let alive = music.playing.is_some_and(|(.., e)| commands.get_entity(e).is_ok());
    if !alive
        && let (Some(i), Some((_, t, _))) = (jukebox.0, music.playing)
        && t == ALL[i]
    {
        jukebox.0 = Some((i + 1) % ALL.len());
    }
    let want = (settings.music > 0).then(|| match jukebox.0 {
        Some(i) if *screen.get() == Screen::Bunker => std::slice::from_ref(&ALL[i]),
        _ => pick(*screen.get(), session.mode, session.stage, fight),
    });
    let same_pool = music.playing.map(|(p, ..)| p) == want;
    if alive && same_pool {
        return;
    }
    let old = music.playing.take().map(|(_, t, e)| {
        if let Ok(mut ec) = commands.get_entity(e) {
            ec.despawn();
        }
        t
    });
    let ended = handover(same_pool, old, &mut music.before);
    if *screen.get() != Screen::Stage {
        music.before = None;
    }
    if let Some(pool) = want {
        let fight = pool == HARD || pool == BOSS;
        let roll = music.rng.next_u32();
        let t = next(pool, ended, (!fight).then(|| session.stage.max(1) - 1), roll);
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
    jukebox: Res<Jukebox>,
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
    let hold = *away
        || match *screen.get() {
            Screen::Stage => paused.is_some_and(|p| p.0),
            Screen::Bunker => jukebox.1,
            _ => false,
        };
    if hold != sink.is_paused() {
        if hold { sink.pause() } else { sink.play() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tracks_follow_the_screen_and_speed_up() {
        let first = |mode, n| next(pick(Screen::Stage, mode, n, Fight::None), None, Some(n - 1), 0);
        assert_eq!(pick(Screen::Settings, Mode::Classic, 1, Fight::None), [Menu]);
        assert_eq!(pick(Screen::CityMap, Mode::City, 1, Fight::None), [MenuAlt]);
        assert_eq!(pick(Screen::Results, Mode::City, 5, Fight::Boss), [Victory]);
        assert_eq!(pick(Screen::GameOver, Mode::Classic, 5, Fight::None), LOSS);
        assert_eq!(pick(Screen::Board, Mode::Classic, 5, Fight::None), LOSS, "game over's track plays on");
        assert_eq!(
            [1, 2, 10, 11, 12, 20, 21, 30, 31].map(|n| first(Mode::Classic, n)),
            [Slow1, Slow2, Slow3, Medium1, Medium2, Medium5, Fast3, Fast6, Slow3]
        );
        assert_eq!(
            (1..=10).map(|d| first(Mode::City, d)).collect::<Vec<_>>(),
            [Slow1, Slow2, Slow3, Medium4, Medium5, Fast1, Fast2, Fast2, Fast3, Fast4]
        );
        assert_eq!(pick(Screen::Stage, Mode::City, 2, Fight::Mini), HARD);
        assert_eq!(pick(Screen::Stage, Mode::City, 9, Fight::Boss), BOSS);
    }

    #[test]
    fn a_second_press_pauses_the_jukebox() {
        let mut j = Jukebox::default();
        j.press(3);
        assert_eq!((j.0, j.1), (Some(3), false));
        j.press(3);
        assert_eq!((j.0, j.1), (Some(3), true), "same track: pause");
        j.press(3);
        assert_eq!((j.0, j.1), (Some(3), false), "and again: resume");
        j.press(3);
        j.press(5);
        assert_eq!((j.0, j.1), (Some(5), false), "another track plays, even from a pause");
    }

    #[test]
    fn after_the_opener_the_pool_shuffles() {
        for pool in [SLOW, CITY_SLOW, CITY_MEDIUM, FAST, HARD, BOSS, LOSS] {
            let mut seen = std::collections::HashSet::new();
            for &ended in pool {
                for roll in 0..20 {
                    let t = next(pool, Some(ended), Some(0), roll);
                    assert!(pool.contains(&t) && t != ended, "never the same track twice in a row");
                    seen.insert(t);
                }
            }
            assert_eq!(seen.len(), pool.len(), "every track comes up");
        }
        assert_eq!(next(BOSS, Some(Fast4), None, 3), BossFight4, "a fight opens on a random track");
        assert_eq!(next(&[Menu], Some(Menu), Some(7), 5), Menu, "single-track pool repeats");
    }

    #[test]
    fn a_fight_ending_moves_the_stage_music_on() {
        // Slow2 plays, a mini-boss cuts in, it dies: the stage pool goes on with Slow3, not back to its opener.
        let mut before = None;
        assert_eq!(handover(false, Some(Slow2), &mut before), None, "fight pool opens fresh");
        let after = handover(false, Some(HardFight1), &mut before);
        assert_eq!(after, Some(Slow2));
        assert_ne!(next(CITY_SLOW, after, Some(0), 0), Slow1, "not back to the opener");
        assert_eq!(handover(true, Some(Slow3), &mut before), Some(Slow3), "same pool: the ended track");
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
