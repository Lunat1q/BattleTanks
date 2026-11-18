//! Gameplay: shared components/resources, screen states and the simulation schedule.
//! Logic entities carry no sprites; `gfx` attaches visuals when they appear.

pub mod combat;
pub mod enemy;
pub mod powerup;
pub mod stage;
pub mod tank;

use crate::geom::Dir;
use crate::input::{Device, DeviceIntents, Intent};
use crate::rules::{self, EnemyKind, GunStats, PowerKind};
use bevy::prelude::*;

#[derive(States, Default, Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Screen {
    #[default]
    Title,
    Stage,
    Results,
    GameOver,
}

pub struct Slot {
    pub device: Device,
    /// Spare tanks.
    pub lives: u32,
    pub score: u32,
    /// Star level 0..=3, lost on death.
    pub level: u8,
    /// No lives left this game.
    pub out: bool,
    /// Kills per `EnemyKind` this stage.
    pub kills: [u32; 4],
}

impl Slot {
    pub fn new(device: Device) -> Self {
        Slot { device, lives: rules::START_LIVES, score: 0, level: 0, out: false, kills: [0; 4] }
    }
}

#[derive(Resource, Default)]
pub struct Session {
    pub slots: [Option<Slot>; 2],
    /// Current stage, 1-based.
    pub stage: usize,
    pub hi_score: u32,
}

impl Session {
    pub fn players(&self) -> usize {
        self.slots.iter().flatten().count()
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
    pub base_alive: bool,
}

#[derive(Resource, Default)]
pub struct Paused(pub bool);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Team {
    Player(usize),
    Enemy,
}

#[derive(Component)]
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
}

impl Tank {
    pub fn new(team: Team, dir: Dir, speed: f32) -> Self {
        Tank { team, dir, want: None, speed, moving: false, blocked: false, slide: 0.0, odometer: 0.0 }
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
    pub hp: u32,
    pub bonus: bool,
    pub turn_in: f32,
    pub fire_in: f32,
}

#[derive(Component)]
pub struct Shield(pub f32);
#[derive(Component)]
pub struct Paralyzed(pub f32);
#[derive(Component)]
pub struct Spawning(pub f32);
/// Short white flash after taking a non-lethal hit.
#[derive(Component)]
pub struct HitFlash(pub f32);

#[derive(Component)]
pub struct Bullet {
    pub owner: Entity,
    pub team: Team,
    pub dir: Dir,
    pub speed: f32,
    pub power: bool,
}

#[derive(Component)]
pub struct Base;

#[derive(Component)]
pub struct PowerUp(pub PowerKind);

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
    Sparks,
    TankBoom,
    BaseBoom,
    Points(u32),
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
        .insert_resource(crate::rng::Rng::from_time())
        .add_message::<Fx>()
        .add_message::<Sfx>()
        .add_message::<combat::Hit>()
        .configure_sets(Update, Sim.run_if(sim_running))
        .add_systems(
            Update,
            (
                stage::spawn_enemies,
                stage::tick_timers,
                tank::player_control,
                enemy::think,
                tank::drive,
                tank::fire,
                combat::fly_bullets,
                combat::resolve_hits,
                powerup::pickup,
                stage::check_end,
            )
                .chain()
                .in_set(Sim),
        )
        .add_plugins(stage::plugin);
}
