//! Rendering: procedural textures, camera, and visuals attached to logic entities.

mod art_icons;
mod art_map;
mod art_tanks;
mod art_world;
pub mod canvas;
mod fx;
mod icon;
mod world;

use crate::rules::{EnemyKind, PowerKind, Rank};
use bevy::camera::ScalingMode;
#[cfg(not(target_os = "android"))]
use bevy::post_process::bloom::Bloom;
use bevy::prelude::*;

/// Draw order (z). Field units: the camera sees 224 units vertically.
pub mod z {
    pub const GROUND: f32 = 0.0;
    pub const FLOOR: f32 = 1.0;
    pub const WALL: f32 = 2.0;
    pub const TANK: f32 = 4.0;
    pub const BULLET: f32 = 5.0;
    pub const TREES: f32 = 6.0;
    pub const FX: f32 = 7.0;
    pub const POWERUP: f32 = 8.0;
    pub const TEXT: f32 = 9.0;
}

pub const VIEW_HEIGHT: f32 = 224.0;

#[derive(Resource)]
pub struct Gfx {
    pub players: Vec<[Handle<Image>; 2]>,
    pub enemies: Vec<[Handle<Image>; 2]>,
    pub mini: [Handle<Image>; 2],
    pub boss: [Handle<Image>; 2],
    pub turret: Handle<Image>,
    /// City Run district-map backdrop (pixel art).
    pub city_backdrop: Handle<Image>,
    pub brick: Handle<Image>,
    pub steel: Handle<Image>,
    pub trees: Handle<Image>,
    pub ice: Handle<Image>,
    pub water: [Handle<Image>; 3],
    pub ground: Handle<Image>,
    pub base: Handle<Image>,
    pub base_dead: Handle<Image>,
    pub bullet: Handle<Image>,
    pub dot: Handle<Image>,
    pub sparkle: Handle<Image>,
    pub ring: Handle<Image>,
    pub flag: Handle<Image>,
    pub powerups: Vec<Handle<Image>>,
}

impl Gfx {
    pub fn powerup(&self, k: PowerKind) -> Handle<Image> {
        self.powerups[PowerKind::ALL.iter().position(|&p| p == k).unwrap_or(0)].clone()
    }
}

pub fn player_color(slot: usize) -> Color {
    if slot == 0 { Color::srgb(1.0, 0.80, 0.32) } else { Color::srgb(0.42, 0.92, 0.52) }
}

pub fn enemy_color(kind: EnemyKind, hp: u32) -> Color {
    match (kind, hp) {
        (EnemyKind::Basic, _) => Color::srgb(0.78, 0.80, 0.84),
        (EnemyKind::Fast, _) => Color::srgb(0.60, 0.80, 0.98),
        (EnemyKind::Power, _) => Color::srgb(0.86, 0.68, 0.98),
        // Never gold/green: those are the players' colours.
        (EnemyKind::Armor, 4) => Color::srgb(0.98, 0.45, 0.72),
        (EnemyKind::Armor, 3) => Color::srgb(0.92, 0.60, 0.80),
        (EnemyKind::Armor, 2) => Color::srgb(0.86, 0.74, 0.84),
        (EnemyKind::Armor, _) => Color::srgb(0.80, 0.80, 0.82),
    }
}

/// City Run turrets and bosses: own reds (never the players' gold/green), fading to grey as they take damage.
pub fn rank_color(rank: Rank, kind: EnemyKind, hp: u32, max_hp: u32) -> Color {
    let full = match rank {
        Rank::Normal => return enemy_color(kind, hp),
        Rank::Turret => Color::srgb(0.88, 0.56, 0.50),
        Rank::Mini => Color::srgb(0.56, 0.48, 1.0),
        Rank::Boss => Color::srgb(0.92, 0.24, 0.32),
    };
    full.mix(&Color::srgb(0.78, 0.78, 0.80), 0.6 * (1.0 - hp as f32 / max_hp.max(1) as f32))
}

pub fn plugin(app: &mut App) {
    // Built right here (not in a startup system) so `Gfx` exists before the first `OnEnter`.
    let gfx = build_textures(&mut app.world_mut().resource_mut::<Assets<Image>>());
    app.insert_resource(gfx).insert_resource(ClearColor(Color::srgb(0.035, 0.04, 0.055))).add_systems(Startup, spawn_camera).add_plugins((
        world::plugin,
        fx::plugin,
        icon::plugin,
    ));
}

fn build_textures(images: &mut Assets<Image>) -> Gfx {
    let mut add = |img: Image| images.add(img);
    let pairs = |v: Vec<[Image; 2]>, add: &mut dyn FnMut(Image) -> Handle<Image>| -> Vec<[Handle<Image>; 2]> {
        v.into_iter().map(|[a, b]| [add(a), add(b)]).collect()
    };
    let players = pairs(art_tanks::players(), &mut add);
    let enemies = pairs(art_tanks::enemies(), &mut add);
    Gfx {
        players,
        enemies,
        mini: art_tanks::mini().map(&mut add),
        boss: art_tanks::boss().map(&mut add),
        turret: add(art_tanks::turret()),
        city_backdrop: add(art_map::city_backdrop()),
        brick: add(art_world::brick()),
        steel: add(art_world::steel()),
        trees: add(art_world::trees()),
        ice: add(art_world::ice()),
        water: [0, 1, 2].map(|f| add(art_world::water(f))),
        ground: add(art_world::ground()),
        base: add(art_world::base(false)),
        base_dead: add(art_world::base(true)),
        bullet: add(art_icons::bullet()),
        dot: add(art_icons::dot()),
        sparkle: add(art_icons::sparkle()),
        ring: add(art_icons::ring()),
        flag: add(art_icons::flag()),
        powerups: PowerKind::ALL.iter().map(|&k| add(art_icons::powerup(k))).collect(),
    }
}

#[derive(Component)]
pub struct MainCamera;

fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        MainCamera,
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::FixedVertical { viewport_height: VIEW_HEIGHT },
            ..OrthographicProjection::default_2d()
        }),
        Transform::from_xyz(104.0, 104.0, 0.0),
        // Sprites are quads with painter-anti-aliased textures, so MSAA adds nothing visible, yet on a TV-class
        // Mali (Sony Bravia, Mali-G57) Bevy's default 4x halved the frame rate.
        Msaa::Off,
        // ponytail: bloom is PC-only. On the Bravia it cost 56 -> 26 fps (34 with 128 px mips); a setting if a strong
        // Android box (e.g. Shield) should get the glow too.
        #[cfg(not(target_os = "android"))]
        Bloom { intensity: 0.12, ..Bloom::NATURAL },
    ));
}
