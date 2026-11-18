//! Gameplay numbers and pure rules. Tune here; everything is in field units (see `geom`) and seconds.

pub const PLAYER_SPEED: f32 = 48.0;
pub const START_LIVES: u32 = 2; // spare tanks, so 3 tanks in total
pub const EXTRA_LIFE_EVERY: u32 = 20_000;
pub const POWERUP_POINTS: u32 = 500;
pub const ENEMIES_PER_STAGE: usize = 20;
/// Enemies number 4, 11 and 18 carry a power-up (they flash red).
pub const BONUS_CARRIERS: [usize; 3] = [3, 10, 17];

pub const SPAWN_TIME: f32 = 1.0; // spawn twinkle before a tank becomes real
pub const RESPAWN_SHIELD: f32 = 3.0;
pub const HELMET_SHIELD: f32 = 10.0;
pub const CLOCK_FREEZE: f32 = 10.0;
pub const SHOVEL_TIME: f32 = 20.0;
pub const PARALYZE_TIME: f32 = 3.0; // a player hit by the other player
pub const SLIDE_TIME: f32 = 0.35; // keep moving this long after releasing on ice
pub const END_DELAY: f32 = 3.0; // stage won / lost -> next screen

pub const SLOW_BULLET: f32 = 140.0;
pub const FAST_BULLET: f32 = 240.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum EnemyKind {
    Basic,
    Fast,
    Power,
    Armor,
}

impl EnemyKind {
    pub const ALL: [EnemyKind; 4] = [EnemyKind::Basic, EnemyKind::Fast, EnemyKind::Power, EnemyKind::Armor];

    pub fn index(self) -> usize {
        self as usize
    }
    pub fn speed(self) -> f32 {
        match self {
            EnemyKind::Basic => 30.0,
            EnemyKind::Fast => 66.0,
            EnemyKind::Power => 40.0,
            EnemyKind::Armor => 32.0,
        }
    }
    pub fn bullet_speed(self) -> f32 {
        match self {
            EnemyKind::Basic | EnemyKind::Fast => SLOW_BULLET,
            EnemyKind::Power => FAST_BULLET,
            EnemyKind::Armor => 190.0,
        }
    }
    pub fn hp(self) -> u32 {
        if self == EnemyKind::Armor { 4 } else { 1 }
    }
    pub fn points(self) -> u32 {
        (self.index() as u32 + 1) * 100
    }
}

/// What a player's gun can do at a star level (0..=3).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct GunStats {
    pub bullet_speed: f32,
    pub max_bullets: usize,
    /// Breaks steel and digs twice as deep into bricks.
    pub power: bool,
}

pub fn player_gun(level: u8) -> GunStats {
    match level {
        0 => GunStats { bullet_speed: SLOW_BULLET, max_bullets: 1, power: false },
        1 => GunStats { bullet_speed: FAST_BULLET, max_bullets: 1, power: false },
        2 => GunStats { bullet_speed: FAST_BULLET, max_bullets: 2, power: false },
        _ => GunStats { bullet_speed: FAST_BULLET, max_bullets: 2, power: true },
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PowerKind {
    Helmet,
    Clock,
    Shovel,
    Star,
    Grenade,
    Tank,
}

impl PowerKind {
    pub const ALL: [PowerKind; 6] =
        [PowerKind::Helmet, PowerKind::Clock, PowerKind::Shovel, PowerKind::Star, PowerKind::Grenade, PowerKind::Tank];
}

/// City Run: what an enemy is beyond its `EnemyKind` (which still sets speed and bullet).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Rank {
    Normal,
    /// Bastion gun on a fortress wall: never moves, aims at the nearest player.
    Turret,
    /// Big tank mid-district; the camera waits until it's dead.
    Mini,
    /// District boss at the top of the map; killing it clears the district.
    Boss,
}

impl Rank {
    /// Side of the square hit box in field units.
    pub fn size(self) -> f32 {
        match self {
            Rank::Normal | Rank::Turret => 16.0,
            Rank::Mini => 24.0,
            Rank::Boss => 32.0,
        }
    }
    /// Hit points in district `d` (1-based); `Normal` uses the kind's own HP.
    pub fn hp(self, kind: EnemyKind, d: usize) -> u32 {
        let d = d as u32;
        match self {
            Rank::Normal => kind.hp(),
            Rank::Turret => 2 + d / 3,
            Rank::Mini => 8 + d,
            Rank::Boss => 20 + 3 * d,
        }
    }
    pub fn points(self, kind: EnemyKind) -> u32 {
        match self {
            Rank::Normal => kind.points(),
            Rank::Turret => 500,
            Rank::Mini => 1000,
            Rank::Boss => 5000,
        }
    }
    pub fn speed(self, kind: EnemyKind) -> f32 {
        match self {
            Rank::Normal => kind.speed(),
            Rank::Turret => 0.0,
            Rank::Mini => 34.0,
            Rank::Boss => 26.0,
        }
    }
    /// Bullets it may have in flight.
    pub fn max_bullets(self) -> usize {
        match self {
            Rank::Normal | Rank::Turret => 1,
            Rank::Mini => 2,
            Rank::Boss => 3,
        }
    }
    /// Seconds between shots, (min, max): later districts shoot more often.
    pub fn fire_gap(self, d: usize) -> (f32, f32) {
        let k = 1.0 - d.min(10) as f32 * 0.04;
        match self {
            Rank::Normal => (0.4, 1.6),
            Rank::Turret => (0.9 * k, 2.2 * k),
            Rank::Mini => (0.35 * k, 1.0 * k),
            Rank::Boss => (0.25 * k, 0.7 * k),
        }
    }
}

pub const CITY_DISTRICTS: usize = 10;
/// Every Nth enemy placed in a district carries a power-up.
pub const CITY_BONUS_EVERY: usize = 5;

/// Seconds between City Run reinforcements from the top of the view.
pub fn reinforce_interval(district: usize, players: usize) -> f32 {
    (7.0 - district as f32 * 0.35 - players.saturating_sub(1) as f32 * 1.0).max(2.5)
}

/// Extra lives earned by going from `old` to `new` points.
pub fn extra_lives(old: u32, new: u32) -> u32 {
    new / EXTRA_LIFE_EVERY - old / EXTRA_LIFE_EVERY
}

/// Seconds between enemy spawns: faster on later stages and with two players.
pub fn spawn_interval(stage: usize, players: usize) -> f32 {
    let base = 3.2 - stage as f32 * 0.06 - (players.saturating_sub(1)) as f32 * 0.4;
    base.max(1.0)
}

/// Enemies allowed on the field at once.
// ponytail: 4 like the classic for 1P; 6 for 2P so co-op stays busy. Tune on the couch.
pub fn max_enemies(players: usize) -> usize {
    if players >= 2 { 6 } else { 4 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extra_life_on_each_threshold() {
        assert_eq!(extra_lives(19_900, 20_000), 1);
        assert_eq!(extra_lives(19_900, 40_100), 2);
        assert_eq!(extra_lives(20_000, 20_400), 0);
    }

    #[test]
    fn guns_grow_with_level() {
        assert_eq!(player_gun(0).max_bullets, 1);
        assert!(player_gun(1).bullet_speed > player_gun(0).bullet_speed);
        assert_eq!(player_gun(2).max_bullets, 2);
        assert!(player_gun(3).power && player_gun(9).power);
    }

    #[test]
    fn spawn_interval_has_a_floor() {
        assert!(spawn_interval(1, 1) > spawn_interval(1, 2));
        assert_eq!(spawn_interval(500, 2), 1.0);
    }

    #[test]
    fn bosses_grow_with_the_district() {
        assert!(Rank::Boss.hp(EnemyKind::Power, 10) > Rank::Boss.hp(EnemyKind::Power, 1));
        assert!(Rank::Mini.hp(EnemyKind::Armor, 1) > EnemyKind::Armor.hp());
        assert_eq!(Rank::Normal.hp(EnemyKind::Armor, 9), 4);
        assert_eq!(Rank::Turret.speed(EnemyKind::Power), 0.0);
        assert!(Rank::Boss.fire_gap(10).1 < Rank::Boss.fire_gap(1).1);
        assert!(reinforce_interval(1, 1) > reinforce_interval(10, 2));
        assert_eq!(reinforce_interval(100, 2), 2.5);
    }

    #[test]
    fn enemy_points_and_hp() {
        assert_eq!(EnemyKind::Basic.points(), 100);
        assert_eq!(EnemyKind::Armor.points(), 400);
        assert_eq!(EnemyKind::Armor.hp(), 4);
    }
}
