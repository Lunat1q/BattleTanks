//! VERSUS (ADR 0019, 0020): two players, two bases, one point-symmetric map. Player 1 defends the bottom base,
//! player 2 the top one. Tanks are unlimited, but each loss waits longer to come back; a round ends when a base
//! falls. A match counts round wins. Each player may bring an NPC army (`Armies`) that drips onto the field from
//! their back corners and fights for their side.

use super::*;
use crate::geom::{FIELD, overlaps, rect_at};
use crate::rng::Rng;
use crate::rules::EnemyKind;

/// Base centres by side: player 1's as in classic, player 2's turned 180 degrees.
pub const BASES: [Vec2; 2] = [Vec2::new(104.0, 8.0), Vec2::new(104.0, FIELD - 8.0)];
/// Each player starts beside their own base (player 1 on the classic spot), facing the other.
pub const SPAWNS: [(Vec2, Dir); 2] = [(Vec2::new(72.0, 8.0), Dir::Up), (Vec2::new(FIELD - 72.0, FIELD - 8.0), Dir::Down)];
/// NPCs enter in the two corners of their side's back row, in turn.
pub const NPC_SPAWNS: [[Vec2; 2]; 2] =
    [[Vec2::new(8.0, 8.0), Vec2::new(FIELD - 8.0, 8.0)], [Vec2::new(FIELD - 8.0, FIELD - 8.0), Vec2::new(8.0, FIELD - 8.0)]];

/// The match so far.
#[derive(Resource, Default)]
pub struct Match {
    /// Rounds won per player.
    pub wins: [u32; 2],
    /// Rounds played, the one on the field included.
    pub round: usize,
    /// The last round: its winner, `None` for a draw.
    pub last: Option<usize>,
    /// NPC army points per player (0 = no armies: a plain duel with timed power-ups).
    pub budget: u32,
}

impl Match {
    /// A new match from PLAY.
    pub fn new(budget: u32) -> Self {
        Match { round: 1, budget, ..default() }
    }
}

/// One side's NPC army this round.
#[derive(Default)]
pub struct Army {
    /// Spawn order (the army shuffled by the gameplay `Rng`, so LAN peers agree).
    pub roster: Vec<EnemyKind>,
    pub next: usize,
    /// Seconds to the next NPC.
    pub timer: f32,
    /// Which back corner is next.
    pub point: usize,
}

/// Both armies, plus the timed power-up clock when there are none.
#[derive(Resource, Default)]
pub struct Armies {
    pub sides: [Army; 2],
    pub drop_in: f32,
}

impl Armies {
    /// The round's armies: each player's `budget` as an AUTO army, shuffled.
    pub fn new(budget: u32, rng: &mut Rng) -> Self {
        let army = |rng: &mut Rng| {
            let mut roster = rules::auto_army(budget);
            rng.shuffle(&mut roster);
            Army { roster, ..default() }
        };
        Armies { sides: [army(rng), army(rng)], drop_in: rules::VERSUS_DROP_GAP }
    }
}

/// Is the `n`th NPC (0-based) of a side a power-up carrier? Every 4th, as in classic.
pub fn carrier(n: usize) -> bool {
    (n + 1).is_multiple_of(rules::NPC_BONUS_EVERY)
}

/// Each side's next NPC every `NPC_SPAWN_GAP` s while fewer than `NPC_MAX_ALIVE` of it are on the field, in a free
/// back corner; without armies, a power-up in the middle band every `VERSUS_DROP_GAP` s.
pub fn spawn_npcs(
    mut commands: Commands,
    time: Res<Time>,
    run: Res<StageRun>,
    session: Res<Session>,
    armies: Option<ResMut<Armies>>,
    tanks: Query<(&Transform, &Tank)>,
    terrain: Res<crate::terrain::Terrain>,
    mut rng: ResMut<Rng>,
    mut sfx: MessageWriter<Sfx>,
) {
    let Some(mut armies) = armies.filter(|_| session.mode == Mode::Versus && run.phase == Phase::Play) else { return };
    let dt = time.delta_secs();
    if armies.sides.iter().all(|a| a.roster.is_empty()) {
        armies.drop_in -= dt;
        if armies.drop_in <= 0.0 {
            armies.drop_in = rules::VERSUS_DROP_GAP;
            let pos = super::powerup::middle_spot(&mut rng, &terrain);
            let kind = rng.pick(&rules::PowerKind::ALL);
            commands.spawn((PowerUp(kind), Transform::from_translation(pos.extend(0.0)), DespawnOnExit(Screen::Stage)));
            sfx.write(Sfx::PowerAppear);
        }
        return;
    }
    for side in 0..2 {
        let army = &mut armies.sides[side];
        if army.next >= army.roster.len() {
            continue;
        }
        army.timer -= dt;
        let alive = tanks.iter().filter(|(_, t)| t.team == Team::Npc(side)).count();
        if army.timer > 0.0 || alive >= rules::NPC_MAX_ALIVE {
            continue;
        }
        let pos = NPC_SPAWNS[side][army.point % 2];
        army.point += 1;
        if tanks.iter().any(|(tf, t)| overlaps(rect_at(pos, 16.0), rect_at(tf.translation.xy(), t.size))) {
            continue; // occupied: the other corner next frame
        }
        let kind = army.roster[army.next];
        let e = super::stage::spawn_enemy(&mut commands, kind, rules::Rank::Normal, pos, carrier(army.next), 0, &mut rng);
        commands.entity(e).insert(Tank::new(Team::Npc(side), SPAWNS[side].1, rules::Rank::Normal.speed(kind)));
        army.next += 1;
        army.timer = rules::NPC_SPAWN_GAP;
    }
}

/// The round is over once a base has fallen: `Some(winner)`, `Some(None)` for a draw (both on the same tick),
/// `None` while it goes on.
pub fn outcome(base_alive: [bool; 2]) -> Option<Option<usize>> {
    match base_alive {
        [true, true] => None,
        [false, false] => Some(None),
        [alive0, _] => Some(Some(usize::from(!alive0))),
    }
}

/// A fresh round: no losses yet, star level 0.
pub fn reset_slots(session: &mut Session) {
    for s in session.slots.iter_mut().flatten() {
        let device = s.device;
        *s = Slot::new(device);
    }
}

/// A lost tank: the player waits out `rules::versus_respawn` (the spawn twinkle included) instead of using a spare.
pub fn lost_tank(slot: &mut Slot) {
    slot.deaths += 1;
    slot.out = true;
    slot.out_wait = rules::versus_respawn(slot.deaths) - rules::SPAWN_TIME;
}

/// A player whose wait is over comes back beside their base (`out_wait` counts down in `stage::borrow_life`).
pub fn respawn(mut commands: Commands, mut session: ResMut<Session>, run: Res<StageRun>) {
    if session.mode != Mode::Versus || run.phase != Phase::Play {
        return;
    }
    for (p, slot) in session.slots.iter_mut().enumerate() {
        if let Some(s) = slot.as_mut().filter(|s| s.out && s.out_wait <= 0.0) {
            s.out = false;
            let (at, facing) = SPAWNS[p];
            super::stage::spawn_player(&mut commands, p, s.level, at, facing);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_round_goes_to_whoever_still_has_a_base() {
        assert_eq!(outcome([true, true]), None);
        assert_eq!(outcome([false, true]), Some(Some(1)), "player 1's base fell");
        assert_eq!(outcome([true, false]), Some(Some(0)));
        assert_eq!(outcome([false, false]), Some(None), "both on one tick: a draw");
    }

    #[test]
    fn each_loss_waits_longer_and_nobody_runs_out() {
        let mut s = Slot::new(crate::input::Device::KeysLeft);
        let waits: Vec<f32> = (0..9)
            .map(|_| {
                lost_tank(&mut s);
                s.out_wait + rules::SPAWN_TIME
            })
            .collect();
        assert_eq!(waits, [1.0, 3.0, 5.0, 7.0, 10.0, 15.0, 20.0, 25.0, 30.0]);
        lost_tank(&mut s);
        assert_eq!(s.out_wait + rules::SPAWN_TIME, 30.0, "capped");
        assert_eq!(s.lives, rules::START_LIVES, "no spares spent");
    }

    #[test]
    fn armies_spend_the_budget_and_flash_every_fourth() {
        use EnemyKind::*;
        assert_eq!(rules::auto_army(20), [Basic, Fast, Power, Armor, Basic, Fast, Power, Basic, Fast]);
        assert_eq!(rules::auto_army(20).iter().map(|k| k.cost()).sum::<u32>(), 20);
        for budget in [0, 1, 10, 30] {
            let spent: u32 = rules::auto_army(budget).iter().map(|k| k.cost()).sum();
            assert_eq!(spent, budget, "budget {budget}");
        }
        let a = Armies::new(20, &mut Rng::new(7));
        let mut sorted = a.sides[0].roster.clone();
        sorted.sort_by_key(|k| k.index());
        let mut auto = rules::auto_army(20);
        auto.sort_by_key(|k| k.index());
        assert_eq!(sorted, auto, "shuffled, same army");
        assert_eq!((0..9).filter(|&n| carrier(n)).collect::<Vec<_>>(), [3, 7]);
        assert_eq!(Team::Npc(1).side(), Team::Player(1).side());
    }

    #[test]
    fn sides_mirror_each_other() {
        for (a, b) in NPC_SPAWNS[0].iter().zip(NPC_SPAWNS[1]) {
            assert_eq!(*a, Vec2::splat(FIELD) - b);
        }
        for side in 0..2 {
            let (spawn, _) = SPAWNS[side];
            let turned = Vec2::splat(FIELD) - SPAWNS[1 - side].0;
            assert_eq!(spawn, turned);
            assert_eq!(BASES[side], Vec2::splat(FIELD) - BASES[1 - side]);
        }
        assert_eq!(SPAWNS[0].0, super::super::stage::PLAYER_SPAWNS[0]);
        assert_eq!(BASES[0], super::super::stage::BASE_POS);
    }
}
