//! VERSUS (ADR 0019): two players, two bases, one point-symmetric map. Player 1 defends the bottom base, player 2
//! the top one. A round ends when a base falls or a player runs out of tanks; a match counts round wins.

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

/// The round is over once a side has lost its base or its last tank: `Some(winner)`, `Some(None)` for a draw
/// (both on the same tick), `None` while it goes on.
pub fn outcome(base_alive: [bool; 2], out: [bool; 2]) -> Option<Option<usize>> {
    match [!base_alive[0] || out[0], !base_alive[1] || out[1]] {
        [false, false] => None,
        [true, true] => Some(None),
        [lost0, _] => Some(Some(usize::from(lost0))),
    }
}

/// A fresh round: everyone back to full tanks at star level 0.
pub fn reset_slots(session: &mut Session, easy_mode: bool) {
    for s in session.slots.iter_mut().flatten() {
        let device = s.device;
        *s = Slot::new(device);
        if easy_mode {
            s.lives += rules::EASY_EXTRA_LIVES;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_round_goes_to_whoever_still_stands() {
        assert_eq!(outcome([true, true], [false, false]), None);
        assert_eq!(outcome([false, true], [false, false]), Some(Some(1)), "player 1's base fell");
        assert_eq!(outcome([true, true], [false, true]), Some(Some(0)), "player 2 is out of tanks");
        assert_eq!(outcome([true, false], [true, false]), Some(None), "both on one tick: a draw");
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
