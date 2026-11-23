//! Gameplay numbers and pure rules. Tune here; everything is in field units (see `geom`) and seconds.

pub const PLAYER_SPEED: f32 = 48.0;
/// City Run: how fast the view may rise (units/s); twice a tank, so following never feels it, only the catch-up
/// once a boss lets go of the camera.
pub const VIEW_CATCH_UP: f32 = 2.0 * PLAYER_SPEED;
pub const START_LIVES: u32 = 2; // spare tanks, so 3 tanks in total
/// City Run districts start with one spare more: they are long and have no stage breaks.
pub const CITY_START_LIVES: u32 = START_LIVES + 1;
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
            EnemyKind::Fast => 54.0,
            EnemyKind::Power => 40.0,
            EnemyKind::Armor => 32.0,
        }
    }
    /// Seconds between unforced heading changes, (min, max). Scouts commit longer so they don't jitter.
    pub fn turn_gap(self) -> (f32, f32) {
        if self == EnemyKind::Fast { (1.4, 4.0) } else { (0.8, 3.0) }
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

/// Highest star level. Level 4 keeps level 3's gun, drives over water and works as armour.
pub const MAX_LEVEL: u8 = 4;
pub const SWIM_LEVEL: u8 = 4;
/// A hit on level-4 armour: this long blinking and untouchable, then the stars drop as if the tank were lost.
pub const ARMOUR_TIME: f32 = 3.0;

/// Does an enemy hit cost this player's stars instead of the tank?
pub fn armour_holds(level: u8) -> bool {
    level >= MAX_LEVEL
}

/// EASY MODE: spare tanks added at the start of a classic run or a district try.
pub const EASY_EXTRA_LIVES: u32 = 2;

/// Shield after (re)spawning; EASY MODE doubles it so a fresh tank isn't shot at the spawn.
pub fn respawn_shield(easy: bool) -> f32 {
    if easy { RESPAWN_SHIELD * 2.0 } else { RESPAWN_SHIELD }
}

/// Star level after losing a tank (or spending level-4 armour): back to 0, or one star less in EASY MODE.
pub fn level_after_loss(level: u8, easy_mode: bool) -> u8 {
    if easy_mode { level.saturating_sub(1) } else { 0 }
}

/// What a player's gun can do at a star level (0..=MAX_LEVEL).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct GunStats {
    pub bullet_speed: f32,
    pub max_bullets: usize,
    /// Breaks steel, digs twice as deep into bricks, and cuts the first trees it flies into.
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
    /// Gun the boss calls in (`game::boss`): never moves, fires all four ways at once, falls with the boss.
    Sentry,
}

/// Seconds a district boss stands still with its row and column lit before its barrage.
pub const BOSS_CHARGE: f32 = 1.2;

/// Seconds between a district boss's barrages: more often further in, and in a rage (`BossKit::rage`) even more.
pub fn boss_barrage_gap(d: usize, rage: bool) -> f32 {
    (8.0 - 0.35 * d.min(10) as f32) * if rage { RAGE_GAP } else { 1.0 }
}

/// The district boss's tricks grow with the district (owner: district 2's boss easier than 6's). Every one is
/// warned and survivable: lit bands, bricks that shoot away, a rage that only shortens the gaps.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct BossKit {
    /// District 3 on: a second barrage `BOSS_ECHO` s after the first, down the same lit bands.
    pub echo: bool,
    /// District 5 on: at 1/3 HP it charges at once, then drives faster and barrages more often.
    pub rage: bool,
    /// District 7 on: at 1/2 HP it raises a brick ring round itself, once.
    pub fortify: bool,
    /// District 9 on: each charge also lights the row and column of the nearest player; shells come down them from the
    /// view's edges with the barrage.
    pub mark: bool,
}

pub fn boss_kit(d: usize) -> BossKit {
    BossKit { echo: d >= 3, rage: d >= 5, fortify: d >= 7, mark: d >= 9 }
}

/// Seconds from a boss barrage to its echo.
pub const BOSS_ECHO: f32 = 0.5;
/// A raging boss: barrage gap and speed factors.
pub const RAGE_GAP: f32 = 0.6;
pub const RAGE_SPEED: f32 = 1.5;

/// A boss's HP phase: 0 full, 1 at 2/3 (sentries), 2 at 1/2 (fortify), 3 at 1/3 (rage).
pub fn boss_phase(hp: u32, max: u32) -> u8 {
    match hp {
        h if h * 3 <= max => 3,
        h if h * 2 <= max => 2,
        h if h * 3 <= max * 2 => 1,
        _ => 0,
    }
}

/// Sentries a district boss keeps up once it is down to 2/3 HP.
pub fn sentry_cap(d: usize) -> usize {
    1 + d / 5
}

impl Rank {
    /// Side of the square hit box in field units.
    pub fn size(self) -> f32 {
        match self {
            Rank::Normal | Rank::Turret | Rank::Sentry => 16.0,
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
            Rank::Sentry => 2 + d / 4,
        }
    }
    pub fn points(self, kind: EnemyKind) -> u32 {
        match self {
            Rank::Normal => kind.points(),
            Rank::Turret => 500,
            Rank::Mini => 1000,
            Rank::Boss => 5000,
            Rank::Sentry => 200,
        }
    }
    pub fn speed(self, kind: EnemyKind) -> f32 {
        match self {
            Rank::Normal => kind.speed(),
            Rank::Turret | Rank::Sentry => 0.0,
            Rank::Mini => 34.0,
            Rank::Boss => 26.0,
        }
    }
    /// Bullets it may have in flight.
    pub fn max_bullets(self) -> usize {
        match self {
            Rank::Normal | Rank::Turret | Rank::Sentry => 1,
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
            Rank::Sentry => (1.6 * k, 2.6 * k),
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

/// Fastest possible classic stage clear, in tenths of a second: the 20th enemy spawns 19 intervals in and becomes
/// real after its twinkle. Integer maths (same formula as `spawn_interval`) so the score server gets the same number.
pub fn fastest_clear_tenths(stage: usize, players: usize) -> u32 {
    let interval = (320 - 6 * stage as i64 - 40 * (players.max(1) as i64 - 1)).max(100); // hundredths
    (19 * interval / 10) as u32 + (SPAWN_TIME * 10.0) as u32
}

/// Par = fastest clear + this; every second under par is worth `TIME_BONUS_PER_SEC`.
pub const PAR_MARGIN_TENTHS: u32 = 1200;
pub const TIME_BONUS_PER_SEC: u32 = 20;

/// Classic stage clear bonus for a clear in `tenths`, rounded down to tens; each player gets it.
pub fn time_bonus(stage: usize, players: usize, tenths: u32) -> u32 {
    let par = fastest_clear_tenths(stage, players) + PAR_MARGIN_TENTHS;
    par.saturating_sub(tenths) * TIME_BONUS_PER_SEC / 100 * 10
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
        assert!(player_gun(3).power && player_gun(MAX_LEVEL).power && player_gun(9).power);
    }

    #[test]
    fn losing_a_tank_costs_stars() {
        assert!(armour_holds(MAX_LEVEL) && !armour_holds(3));
        assert_eq!(level_after_loss(MAX_LEVEL, false), 0);
        assert_eq!(level_after_loss(MAX_LEVEL, true), 3, "EASY MODE costs one star");
        assert_eq!(level_after_loss(0, true), 0);
        assert!(respawn_shield(true) > respawn_shield(false));
    }

    #[test]
    fn spawn_interval_has_a_floor() {
        assert!(spawn_interval(1, 1) > spawn_interval(1, 2));
        assert_eq!(spawn_interval(500, 2), 1.0);
    }

    #[test]
    fn time_bonus_rewards_fast_clears() {
        let fastest = fastest_clear_tenths(1, 1);
        assert!(((19.0 * spawn_interval(1, 1) + SPAWN_TIME) * 10.0 - fastest as f32).abs() < 1.0);
        assert_eq!(fastest_clear_tenths(999, 2), 200, "floor of 1 s per spawn");
        assert_eq!(time_bonus(1, 1, fastest), 2400);
        assert_eq!(time_bonus(1, 1, fastest + 605), 1190, "60.5 s slower");
        assert_eq!(time_bonus(1, 1, fastest + PAR_MARGIN_TENTHS), 0);
        assert_eq!(time_bonus(1, 1, 99_999), 0);
        // Same vector as the score server's RunRulesTests.Shares_its_numbers_with_the_game (score_server repo).
        assert_eq!((fastest_clear_tenths(7, 2), time_bonus(7, 2, 1500)), (462, 320));
    }

    #[test]
    fn boss_tricks_pile_up_by_district() {
        let tricks = |d| {
            let k = boss_kit(d);
            [k.echo, k.rage, k.fortify, k.mark].iter().filter(|&&t| t).count()
        };
        assert_eq!(tricks(1), 0);
        assert_eq!(tricks(2), 0, "district 2 keeps the plain boss");
        assert!(tricks(6) > tricks(2));
        assert_eq!(tricks(10), 4);
        assert!((1..10).all(|d| tricks(d) <= tricks(d + 1)), "never fewer further in");
        assert!(boss_barrage_gap(10, true) > 2.0 * BOSS_CHARGE, "time to shoot back between barrages");
        assert_eq!([30, 20, 15, 10, 1].map(|h| boss_phase(h, 30)), [0, 1, 2, 3, 3]);
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

    #[test]
    fn scouts_are_quick_but_not_twitchy() {
        let fastest = EnemyKind::ALL.iter().max_by(|a, b| a.speed().total_cmp(&b.speed())).unwrap();
        assert_eq!(*fastest, EnemyKind::Fast);
        assert!(EnemyKind::Fast.speed() < 1.2 * PLAYER_SPEED, "a player can still chase one down");
        assert!(EnemyKind::Fast.turn_gap().0 > EnemyKind::Basic.turn_gap().0);
    }
}
