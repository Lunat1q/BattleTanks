//! VERSUS (ADR 0019, 0020): two players, two bases, one point-symmetric map. Player 1 defends the bottom base,
//! player 2 the top one. Tanks are unlimited, but each loss waits longer to come back; a round ends when a base
//! falls. A match counts round wins.

use super::*;
use crate::geom::FIELD;

/// Base centres by side: player 1's as in classic, player 2's turned 180 degrees.
pub const BASES: [Vec2; 2] = [Vec2::new(104.0, 8.0), Vec2::new(104.0, FIELD - 8.0)];
/// Each player starts beside their own base (player 1 on the classic spot), facing the other.
pub const SPAWNS: [(Vec2, Dir); 2] = [(Vec2::new(72.0, 8.0), Dir::Up), (Vec2::new(FIELD - 72.0, FIELD - 8.0), Dir::Down)];

/// The match so far.
#[derive(Resource, Default)]
pub struct Match {
    /// Rounds won per player.
    pub wins: [u32; 2],
    /// Rounds played, the one on the field included.
    pub round: usize,
    /// The last round: its winner, `None` for a draw.
    pub last: Option<usize>,
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
    fn sides_mirror_each_other() {
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
