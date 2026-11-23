//! Gameplay: shared components/resources, screen states and the simulation schedule.
//! Logic entities carry no sprites; `gfx` attaches visuals when they appear.

pub mod boss;
pub mod city;
pub mod cloud;
pub mod combat;
pub mod enemy;
pub mod powerup;
pub mod progress;
pub mod stage;
pub mod tank;
pub mod versus;

pub use crate::achieve::Feat;
use crate::geom::Dir;
use crate::input::{Device, DeviceIntents, Intent};
use crate::rules::{self, EnemyKind, GunStats, PowerKind, Rank};
use bevy::prelude::*;

#[derive(States, Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Screen {
    #[default]
    Title,
    /// PLAY from the title: a card per mode.
    Play,
    Stage,
    Results,
    GameOver,
    Settings,
    /// City Run district map: pick an unlocked district.
    CityMap,
    /// Classic stage tiles: pick an unlocked stage.
    StageSelect,
    /// World board: name entry and rank, after a classic game over or a City Run district clear.
    Board,
    /// HI SCORES from the title: the world board's top 10 per category.
    HiScores,
    /// LAN CO-OP from the title: host a game or join one on the same Wi-Fi.
    Lan,
    /// ACHIEVEMENTS from the title (hosts with `Platform::achieve`).
    Achievements,
    /// NEW ACHIEVEMENTS after a run that unlocked some, before the world board.
    Unlocks,
    /// The secret bunker (a button combo on the title): changelog, jukebox, sound test.
    Bunker,
    /// After a VERSUS round: who won it, round wins, kills.
    VersusResults,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Mode {
    /// 30 single-screen stages defending the base.
    #[default]
    Classic,
    /// 10 tall scrolling districts, each ending in a boss.
    City,
    /// Two players, two bases, one point-symmetric map (ADR 0019).
    Versus,
}

pub struct Slot {
    pub device: Device,
    /// Spare tanks.
    pub lives: u32,
    pub score: u32,
    /// Star level 0..=`rules::MAX_LEVEL`, lost on death.
    pub level: u8,
    /// No lives left this game.
    pub out: bool,
    /// Seconds of play left before an out player may borrow (`rules::BORROW_WAIT`).
    pub out_wait: f32,
    /// Kills per `EnemyKind` this stage.
    pub kills: [u32; 4],
    /// Power-ups picked up this stage.
    pub pickups: u32,
    /// City Run kills this district beyond `kills`: turrets, mini-bosses, the boss.
    pub big: [u32; 3],
    /// VERSUS: the other player's tanks destroyed this round.
    pub frags: u32,
}

impl Slot {
    pub fn new(device: Device) -> Self {
        Slot {
            device,
            lives: rules::START_LIVES,
            score: 0,
            level: 0,
            out: false,
            out_wait: 0.0,
            kills: [0; 4],
            pickups: 0,
            big: [0; 3],
            frags: 0,
        }
    }
}

#[derive(Resource, Default)]
pub struct Session {
    pub slots: [Option<Slot>; 2],
    pub mode: Mode,
    /// Current stage (Classic) or district (City), 1-based.
    pub stage: usize,
    pub hi_score: u32,
    /// Stages (classic run) or districts (City Run) played since the title; the score server checks it.
    pub log: Vec<StageLog>,
    /// Time bonus of the stage just cleared (classic), shown in the results tally.
    pub bonus: u32,
}

/// One stage (classic) or district (City Run) played. `kills` / `pickups` / `big` per joined player, in slot order.
#[derive(Clone, Debug, PartialEq)]
pub struct StageLog {
    pub stage: usize,
    /// Play time to the last kill (or to the end, if lost), tenths of a second.
    pub tenths: u32,
    pub won: bool,
    pub kills: Vec<[u32; 4]>,
    pub pickups: Vec<u32>,
    /// City Run: turrets, mini-bosses, boss.
    pub big: Vec<[u32; 3]>,
}

impl Session {
    pub fn players(&self) -> usize {
        self.slots.iter().flatten().count()
    }

    /// Sum of the joined players' scores (what the online board ranks).
    pub fn team_score(&self) -> u32 {
        self.slots.iter().flatten().map(|s| s.score).sum()
    }

    /// Adds points; returns extra lives earned.
    pub fn add_score(&mut self, slot: usize, points: u32) -> u32 {
        let Some(s) = self.slots[slot].as_mut() else { return 0 };
        let extra = rules::extra_lives(s.score, s.score + points);
        s.lives += extra;
        s.score += points;
        self.hi_score = self.hi_score.max(s.score);
        extra
    }

    /// Player `p` has been out `rules::BORROW_WAIT` and the partner has a spare tank to give.
    pub fn can_borrow(&self, p: usize) -> bool {
        self.slots[p].as_ref().is_some_and(|s| s.out && s.out_wait <= 0.0) && self.slots[1 - p].as_ref().is_some_and(|s| s.lives > 0)
    }

    /// Player `p` takes one of the partner's spare tanks and is back in. False if there is none to take.
    pub fn borrow(&mut self, p: usize) -> bool {
        if !self.can_borrow(p) {
            return false;
        }
        if let Some(mate) = self.slots[1 - p].as_mut() {
            mate.lives -= 1;
        }
        if let Some(me) = self.slots[p].as_mut() {
            me.out = false;
        }
        true
    }

    /// This frame's intent for a player slot (empty if the slot is unused).
    pub fn intent(&self, slot: usize, devices: &DeviceIntents) -> Intent {
        self.slots[slot].as_ref().map(|s| devices.get(s.device)).unwrap_or_default()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    /// Stage banner, nothing moves.
    Intro,
    Play,
    /// All enemies gone; short victory lap before results.
    Won,
    /// Base destroyed or everyone out; world keeps running briefly.
    Lost,
}

#[derive(Resource)]
pub struct StageRun {
    pub phase: Phase,
    pub timer: f32,
    pub roster: Vec<EnemyKind>,
    pub next_enemy: usize,
    pub spawn_timer: f32,
    pub spawn_point: usize,
    pub freeze: f32,
    pub shovel: f32,
    /// Per side: player 1's base (bottom), player 2's (top, VERSUS only).
    pub base_alive: [bool; 2],
    /// Seconds in `Phase::Play` (pauses and banners excluded): the clear time.
    pub clock: f32,
}

#[derive(Resource, Default)]
pub struct Paused(pub bool);

/// Visible slice of the field: `bottom..bottom + FIELD`. Always 0 on a classic stage; City Run scrolls it.
/// Tanks can't leave it and bullets pop at its edges.
#[derive(Resource, Default)]
pub struct View {
    pub bottom: f32,
}

impl View {
    pub fn top(&self) -> f32 {
        self.bottom + crate::geom::FIELD
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Team {
    Player(usize),
    Enemy,
}

impl Team {
    /// VERSUS side (0 bottom, 1 top); the classic enemy has none.
    pub fn side(self) -> Option<usize> {
        match self {
            Team::Player(p) => Some(p),
            Team::Enemy => None,
        }
    }
}

/// Spawn order of a simulated thing (tank, bullet, power-up, base), counted from 1 per process. Sim loops whose
/// result depends on order walk entities by it: Bevy's own order follows archetypes, and those differ between two
/// LAN machines because `gfx` adds sprites on different frames.
#[derive(Component, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct NetId(pub u64);

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl Default for NetId {
    fn default() -> Self {
        NetId(NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

/// Spawn-order lookup for `sort_by_key::<Entity, _>(|e| order(&ids, *e))`.
pub type Ids<'w, 's> = Query<'w, 's, &'static NetId>;

pub fn order(ids: &Ids, e: Entity) -> NetId {
    ids.get(e).copied().unwrap_or(NetId(u64::MAX))
}

#[derive(Component)]
#[require(NetId)]
pub struct Tank {
    pub team: Team,
    pub dir: Dir,
    /// Direction the driver wants this frame (`None` = stop).
    pub want: Option<Dir>,
    pub speed: f32,
    pub moving: bool,
    /// Could not move the full distance last frame.
    pub blocked: bool,
    /// Ice slide time left.
    pub slide: f32,
    /// Distance driven, drives tread animation.
    pub odometer: f32,
    /// Side of the square hit box (16; City Run bosses are bigger).
    pub size: f32,
}

impl Tank {
    pub fn new(team: Team, dir: Dir, speed: f32) -> Self {
        Tank { team, dir, want: None, speed, moving: false, blocked: false, slide: 0.0, odometer: 0.0, size: crate::geom::TILE }
    }
}

#[derive(Component)]
pub struct Gun {
    pub stats: GunStats,
    pub trigger: bool,
    pub cooldown: f32,
}

#[derive(Component)]
pub struct Enemy {
    pub kind: EnemyKind,
    pub rank: Rank,
    pub hp: u32,
    pub max_hp: u32,
    pub bonus: bool,
    pub turn_in: f32,
    pub fire_in: f32,
    /// District boss: seconds to its next barrage, below 0 while it charges (`boss`).
    pub skill: f32,
    /// District boss: HP phases already acted on (`rules::boss_phase`).
    pub phase: u8,
    /// District boss: the player's row and column it marked when this charge began (`BossKit::mark`).
    pub mark: Option<Vec2>,
}

#[derive(Component)]
pub struct Shield(pub f32);
#[derive(Component)]
pub struct Paralyzed(pub f32);
/// Level-4 armour took a hit: untouchable and blinking for the time left, then stars drop (sunk if on water).
#[derive(Component)]
pub struct Armour(pub f32);
#[derive(Component)]
pub struct Spawning(pub f32);
/// Short white flash after taking a non-lethal hit.
#[derive(Component)]
pub struct HitFlash(pub f32);

#[derive(Component)]
#[require(NetId)]
pub struct Bullet {
    pub owner: Entity,
    pub team: Team,
    pub dir: Dir,
    pub speed: f32,
    pub power: bool,
    /// A power shell that hasn't cut into trees yet (it cuts only the first ones it meets).
    pub trees: bool,
}

/// A base to defend; the number is its side (0 bottom centre, 1 top centre in VERSUS).
#[derive(Component, Clone, Copy)]
#[require(NetId)]
pub struct Base(pub usize);

#[derive(Component)]
#[require(NetId)]
pub struct PowerUp(pub PowerKind);

/// A City Run power-up placed in a brick stash: stays put when a new drop appears.
#[derive(Component)]
pub struct Stash;

/// Visual event for `gfx`.
#[derive(Message, Clone, Copy)]
pub struct Fx {
    pub kind: FxKind,
    pub pos: Vec2,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum FxKind {
    Muzzle(Dir),
    BulletPop,
    Bricks,
    Leaves,
    Sparks,
    TankBoom,
    BaseBoom,
    Points(u32),
}

/// Controller vibration for a player slot; `input` sends it to that slot's pad if vibration is on.
#[derive(Message, Clone, Copy, PartialEq, Debug)]
pub struct Rumble {
    pub slot: usize,
    pub strong: f32,
    pub weak: f32,
    pub secs: f32,
    /// Seconds to wait before it starts (lets one event play a pattern of pulses).
    pub delay: f32,
}

impl Rumble {
    fn new(slot: usize, strong: f32, weak: f32, secs: f32) -> Self {
        Rumble { slot, strong, weak, secs, delay: 0.0 }
    }
    /// Own tank destroyed: a "sad" falling wah-wah-wahhh, three pulses, each softer, the last one long.
    pub fn death(slot: usize) -> [Self; 3] {
        [
            Rumble::new(slot, 0.85, 0.5, 0.22),
            Rumble { delay: 0.34, ..Rumble::new(slot, 0.55, 0.3, 0.22) },
            Rumble { delay: 0.68, ..Rumble::new(slot, 0.35, 0.15, 0.6) },
        ]
    }
    /// Base destroyed: everyone feels it.
    pub fn base_lost(slot: usize) -> Self {
        Rumble::new(slot, 1.0, 1.0, 0.9)
    }
    /// Paralyzed by the other player.
    pub fn stunned(slot: usize) -> Self {
        Rumble::new(slot, 0.4, 0.5, 0.3)
    }
    /// Our shell hit an enemy that survived it (armour, turret, boss).
    pub fn hit(slot: usize) -> Self {
        Rumble::new(slot, 0.2, 0.35, 0.07)
    }
    /// Our shell killed an ordinary enemy.
    pub fn kill(slot: usize) -> Self {
        Rumble::new(slot, 0.45, 0.6, 0.15)
    }
    /// A mini-boss or the boss went down.
    pub fn big_kill(slot: usize) -> Self {
        Rumble::new(slot, 1.0, 0.8, 0.4)
    }
    /// Small confirmation: a power-up.
    pub fn tick(slot: usize) -> Self {
        Rumble::new(slot, 0.0, 0.45, 0.12)
    }
}

/// Sound event for `audio`.
#[derive(Message, Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Sfx {
    Shoot,
    Brick,
    Steel,
    ArmorHit,
    EnemyBoom,
    PlayerBoom,
    BaseBoom,
    PowerAppear,
    PowerPick,
    ExtraLife,
    Pause,
    StageStart,
    GameOver,
    MenuMove,
    MenuPick,
    GateOpen,
    BossBoom,
    /// District boss starts charging a barrage.
    BossCharge,
    Barrage,
}

#[derive(SystemSet, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Sim;

/// The world simulates: on a stage, not paused, past the intro banner.
pub fn sim_running(screen: Res<State<Screen>>, paused: Res<Paused>, run: Option<Res<StageRun>>) -> bool {
    *screen.get() == Screen::Stage && !paused.0 && run.is_some_and(|r| r.phase != Phase::Intro)
}

pub fn plugin(app: &mut App) {
    app.init_state::<Screen>()
        .init_resource::<Session>()
        .init_resource::<Paused>()
        .init_resource::<View>()
        .insert_resource(crate::rng::Rng::from_time())
        .add_message::<Fx>()
        .add_message::<Sfx>()
        .add_message::<combat::Hit>()
        .add_message::<Rumble>()
        .add_message::<Feat>()
        .configure_sets(Update, Sim.run_if(sim_running))
        .add_systems(
            Update,
            (
                stage::spawn_enemies,
                stage::tick_timers,
                stage::borrow_life,
                tank::player_control,
                enemy::think,
                boss::act,
                tank::drive,
                tank::fire,
                combat::fly_bullets,
                combat::resolve_hits,
                combat::armour_wears_off,
                powerup::pickup,
                stage::check_end,
            )
                .chain()
                .in_set(Sim),
        )
        .add_plugins((stage::plugin, city::plugin, progress::plugin));
}
