//! What happened in play (`Feat`, written by the sim) -> unlocks and counters in the `Record`. Pure: it only reads
//! the game, so LAN lockstep hashes never see it. Only this device's slots count (couch: both, LAN: `net.me()`).

use super::Record;
use crate::game::Mode;
use crate::rules::{EnemyKind, Rank};
use bevy::prelude::Message;

#[derive(Message, Clone, Copy, PartialEq, Debug)]
pub enum Feat {
    /// A stage or district starts; `fresh` = a new run (classic: the first stage since the title; every district).
    Start {
        mode: Mode,
        stage: usize,
        joined: [bool; 2],
        fresh: bool,
    },
    /// `slot`'s shell destroyed an enemy (grenade kills are `Grenade`).
    Kill {
        slot: usize,
        kind: EnemyKind,
        rank: Rank,
    },
    /// `slot` lost a tank; `stunned` = while paralyzed by the partner.
    Lost {
        slot: usize,
        stunned: bool,
    },
    /// `slot` and the enemy that killed it destroyed each other in the same frame.
    Mutual {
        slot: usize,
    },
    /// `by` stunned the partner.
    Stun {
        by: usize,
    },
    /// `slot` took one of the partner's spares; `partner_left` spares are left.
    Borrow {
        slot: usize,
        partner_left: u32,
    },
    /// `slot` picked up a star, now at `level`.
    Star {
        slot: usize,
        level: u8,
    },
    /// `slot`'s grenade destroyed `count` enemies.
    Grenade {
        slot: usize,
        count: u32,
    },
    /// `slot`'s shell cancelled an enemy shell.
    Cancel {
        slot: usize,
    },
    /// `slot`'s shell hit the base.
    OwnBase {
        slot: usize,
    },
    Secret,
    Pause {
        slot: usize,
    },
    /// A classic score after a stage ends (won or lost).
    Score {
        slot: usize,
        score: u32,
    },
    /// Stage / district cleared: who is out, and (classic) whether every brick of the base ring is gone.
    Won {
        out: [bool; 2],
        ring_gone: bool,
    },
}

#[derive(Default)]
pub struct Tracker {
    mode: Mode,
    stage: usize,
    joined: [bool; 2],
    /// This machine's slot in a LAN session.
    me: Option<usize>,
    // Per run.
    streak: [u32; 2],
    clean_run: [u32; 2],
    borrows: [u32; 2],
    // Per stage.
    lost: [u32; 2],
    kills: [u32; 2],
    stuns: [u32; 2],
    pauses: [u32; 2],
    stars: [u32; 2],
    secrets: u32,
    /// Stage clock when the slot took the partner's last spare.
    last_spare: [Option<f32>; 2],
}

impl Tracker {
    pub fn seat(&mut self, me: Option<usize>) {
        self.me = me;
    }

    fn local(&self, s: usize) -> bool {
        self.joined[s] && self.me.is_none_or(|m| m == s)
    }

    /// Credits `key` to slot `s` if that player plays on this device.
    fn give(&self, rec: &mut Record, s: usize, key: &str) {
        if self.local(s) {
            rec.unlock(key);
        }
    }

    /// One feat; `clock` = the stage's play clock in seconds.
    pub fn feat(&mut self, f: Feat, clock: f32, rec: &mut Record) {
        match f {
            Feat::Start { mode, stage, joined, fresh } => {
                let me = self.me;
                let run = (self.streak, self.clean_run, self.borrows);
                *self = Tracker { mode, stage, joined, me, ..Tracker::default() };
                if !fresh {
                    (self.streak, self.clean_run, self.borrows) = run;
                }
            }
            Feat::Kill { slot, kind, rank } => {
                let local = self.local(slot);
                match rank {
                    Rank::Normal => {
                        self.kills[slot] += 1;
                        self.streak[slot] += 1;
                        if local {
                            rec.unlock("first_blood");
                            for k in ["kills_100", "kills_1000", "kills_5000"] {
                                rec.add(k, 1);
                            }
                            if kind == EnemyKind::Armor {
                                rec.add("can_opener", 1);
                            }
                            for n in [10, 25, 50] {
                                if self.streak[slot] >= n {
                                    rec.unlock(&format!("streak_{n}"));
                                }
                            }
                        }
                    }
                    Rank::Turret if local => rec.add("wallbreaker", 1),
                    Rank::Boss if local => rec.add("boss_slayer", 1),
                    _ => {}
                }
            }
            Feat::Lost { slot, stunned } => {
                self.lost[slot] += 1;
                self.streak[slot] = 0;
                self.clean_run[slot] = 0;
                if stunned {
                    self.give(rec, 1 - slot, "friends_like_these");
                }
                if self.last_spare[slot].take().is_some_and(|t| clock - t <= 10.0) {
                    self.give(rec, slot, "borrowed_time");
                }
            }
            Feat::Mutual { slot } => self.give(rec, slot, "mutual_destruction"),
            Feat::Stun { by } => {
                self.stuns[by] += 1;
                if self.stuns[by] >= 10 {
                    self.give(rec, by, "ff_enthusiast");
                }
            }
            Feat::Borrow { slot, partner_left } => {
                self.borrows[slot] += 1;
                self.give(rec, slot, "lifeline");
                if self.borrows[slot] >= 5 {
                    self.give(rec, slot, "loan_shark");
                }
                if partner_left == 0 {
                    self.last_spare[slot] = Some(clock);
                }
            }
            Feat::Star { slot, level } => {
                self.stars[slot] += 1;
                if level >= 3 {
                    self.give(rec, slot, "fully_loaded");
                }
            }
            Feat::Grenade { slot, count } => {
                let keys: &[&str] = match count {
                    0 => &["overkill"],
                    1..4 => &[],
                    4..6 => &["clean_sweep"],
                    _ => &["clean_sweep", "six_pack"],
                };
                keys.iter().for_each(|k| self.give(rec, slot, k));
            }
            Feat::Cancel { slot } if self.local(slot) => rec.add("bullet_ballet", 1),
            Feat::Cancel { .. } => {}
            Feat::OwnBase { slot } => self.give(rec, slot, "own_goal"),
            // Whoever broke in: a team find (the record is the device's anyway).
            Feat::Secret => {
                self.secrets += 1;
                rec.unlock("digger");
                if self.secrets >= 3 {
                    rec.unlock("treasure_hunter");
                }
            }
            Feat::Pause { slot } => {
                self.pauses[slot] += 1;
                if self.pauses[slot] >= 10 {
                    self.give(rec, slot, "bathroom_break");
                }
            }
            Feat::Score { slot, score } if self.mode == Mode::Classic => {
                for (n, k) in [(20_000, "score_20k"), (50_000, "score_50k"), (100_000, "score_100k")] {
                    if score >= n {
                        self.give(rec, slot, k);
                    }
                }
            }
            Feat::Score { .. } => {}
            Feat::Won { out, ring_gone } => self.won(out, ring_gone, rec),
        }
    }

    fn won(&mut self, out: [bool; 2], ring_gone: bool, rec: &mut Record) {
        let two = self.joined == [true, true];
        let clean = |s: usize| !out[s] && self.lost[s] == 0;
        let any_clean = (0..2).any(|s| self.local(s) && clean(s));
        if self.me.is_some() {
            rec.unlock("lan_party");
        }
        if self.mode == Mode::City {
            rec.unlock(&format!("city_{}", self.stage));
            if two {
                rec.unlock("duo_city");
                (0..2).filter(|&s| self.stars[s] > 0 && self.stars[1 - s] == 0).for_each(|s| self.give(rec, s, "star_thief"));
            }
            if any_clean {
                rec.unlock("untouchable");
                if self.stage == crate::rules::CITY_DISTRICTS {
                    rec.unlock("ghost");
                }
            }
            return;
        }
        for (n, k) in [(1, "stage_1"), (10, "stage_10"), (20, "stage_20"), (30, "stage_30"), (31, "second_lap")] {
            if self.stage == n {
                rec.unlock(k);
            }
        }
        if self.stage == 20 && !two {
            rec.unlock("lone_wolf");
        }
        if ring_gone {
            rec.unlock("hanging_by_a_thread");
        }
        if any_clean {
            rec.unlock("flawless");
        }
        for s in 0..2 {
            self.clean_run[s] = if clean(s) { self.clean_run[s] + 1 } else { 0 };
            if self.clean_run[s] >= 5 {
                self.give(rec, s, "iron_wall");
            }
        }
        if two {
            rec.unlock("duo_stage");
            for s in 0..2 {
                if self.kills[s] == 0 && self.kills[1 - s] >= 20 {
                    self.give(rec, s, "conscientious_objector");
                }
                if out[s] && !out[1 - s] {
                    self.give(rec, s, "backseat_driver");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOTH: [bool; 2] = [true, true];
    const SOLO: [bool; 2] = [true, false];

    fn start(t: &mut Tracker, rec: &mut Record, mode: Mode, stage: usize, joined: [bool; 2], fresh: bool) {
        t.feat(Feat::Start { mode, stage, joined, fresh }, 0.0, rec);
    }

    fn kill(t: &mut Tracker, rec: &mut Record, slot: usize, n: u32) {
        for _ in 0..n {
            t.feat(Feat::Kill { slot, kind: EnemyKind::Basic, rank: Rank::Normal }, 0.0, rec);
        }
    }

    fn won(t: &mut Tracker, rec: &mut Record, out: [bool; 2]) {
        t.feat(Feat::Won { out, ring_gone: false }, 0.0, rec);
    }

    #[test]
    fn counters_climb_and_unlock_at_their_target() {
        let (mut t, mut rec) = (Tracker::default(), Record::default());
        start(&mut t, &mut rec, Mode::Classic, 1, SOLO, true);
        kill(&mut t, &mut rec, 0, 100);
        assert!(rec.done.contains("first_blood") && rec.done.contains("kills_100"));
        assert_eq!((rec.steps["kills_1000"], rec.steps["kills_100"]), (100, 100));
        t.feat(Feat::Kill { slot: 0, kind: EnemyKind::Armor, rank: Rank::Normal }, 0.0, &mut rec);
        t.feat(Feat::Kill { slot: 0, kind: EnemyKind::Basic, rank: Rank::Boss }, 0.0, &mut rec);
        assert_eq!((rec.steps["can_opener"], rec.steps["boss_slayer"], rec.steps["kills_1000"]), (1, 1, 101), "a boss isn't a kill");
    }

    #[test]
    fn streaks_run_across_stages_and_end_with_a_loss_or_a_new_run() {
        let (mut t, mut rec) = (Tracker::default(), Record::default());
        start(&mut t, &mut rec, Mode::Classic, 1, SOLO, true);
        kill(&mut t, &mut rec, 0, 6);
        start(&mut t, &mut rec, Mode::Classic, 2, SOLO, false);
        kill(&mut t, &mut rec, 0, 4);
        assert!(rec.done.contains("streak_10"));
        t.feat(Feat::Lost { slot: 0, stunned: false }, 0.0, &mut rec);
        kill(&mut t, &mut rec, 0, 24);
        start(&mut t, &mut rec, Mode::Classic, 1, SOLO, true);
        kill(&mut t, &mut rec, 0, 1);
        assert!(!rec.done.contains("streak_25"), "lost a tank, then a new run");
    }

    #[test]
    fn over_lan_only_this_machines_slot_counts() {
        let (mut t, mut rec) = (Tracker::default(), Record::default());
        t.seat(Some(1));
        start(&mut t, &mut rec, Mode::Classic, 1, BOTH, true);
        kill(&mut t, &mut rec, 0, 3);
        t.feat(Feat::Star { slot: 0, level: 3 }, 0.0, &mut rec);
        assert!(rec.done.is_empty() && rec.steps.is_empty(), "the partner's kills and stars are theirs");
        kill(&mut t, &mut rec, 1, 1);
        won(&mut t, &mut rec, [false, false]);
        for k in ["first_blood", "stage_1", "duo_stage", "lan_party", "flawless"] {
            assert!(rec.done.contains(k), "{k}");
        }
    }

    #[test]
    fn classic_clears() {
        let (mut t, mut rec) = (Tracker::default(), Record::default());
        for stage in 16..=20 {
            start(&mut t, &mut rec, Mode::Classic, stage, SOLO, stage == 16);
            won(&mut t, &mut rec, [false, false]);
        }
        assert!(["stage_20", "lone_wolf", "iron_wall", "flawless"].iter().all(|k| rec.done.contains(k)));
        let (mut t, mut rec) = (Tracker::default(), Record::default());
        start(&mut t, &mut rec, Mode::Classic, 31, BOTH, true);
        kill(&mut t, &mut rec, 1, 20);
        t.feat(Feat::Lost { slot: 1, stunned: false }, 0.0, &mut rec);
        t.feat(Feat::Won { out: [true, false], ring_gone: true }, 0.0, &mut rec);
        for k in ["second_lap", "conscientious_objector", "backseat_driver", "hanging_by_a_thread", "duo_stage"] {
            assert!(rec.done.contains(k), "{k}");
        }
        assert!(!rec.done.contains("flawless") && !rec.done.contains("lone_wolf"), "1P was out, 2P lost one");
    }

    #[test]
    fn city_clears() {
        let (mut t, mut rec) = (Tracker::default(), Record::default());
        start(&mut t, &mut rec, Mode::City, 10, BOTH, true);
        t.feat(Feat::Star { slot: 1, level: 1 }, 0.0, &mut rec);
        (0..3).for_each(|_| t.feat(Feat::Secret, 0.0, &mut rec));
        won(&mut t, &mut rec, [false, false]);
        for k in ["city_10", "duo_city", "star_thief", "untouchable", "ghost", "digger", "treasure_hunter"] {
            assert!(rec.done.contains(k), "{k}");
        }
        assert!(!rec.done.contains("stage_10"));
    }

    #[test]
    fn partner_pranks_are_credited_to_who_did_them() {
        let (mut t, mut rec) = (Tracker::default(), Record::default());
        t.seat(Some(0));
        start(&mut t, &mut rec, Mode::Classic, 1, BOTH, true);
        t.feat(Feat::Lost { slot: 1, stunned: true }, 0.0, &mut rec);
        assert!(rec.done.contains("friends_like_these"), "1P stunned 2P, an enemy finished it");
        (0..10).for_each(|_| t.feat(Feat::Stun { by: 1 }, 0.0, &mut rec));
        (0..9).for_each(|_| t.feat(Feat::Pause { slot: 0 }, 0.0, &mut rec));
        assert!(!rec.done.contains("ff_enthusiast") && !rec.done.contains("bathroom_break"));
        t.feat(Feat::Pause { slot: 0 }, 0.0, &mut rec);
        t.feat(Feat::OwnBase { slot: 0 }, 0.0, &mut rec);
        t.feat(Feat::Mutual { slot: 0 }, 0.0, &mut rec);
        assert!(["bathroom_break", "own_goal", "mutual_destruction"].iter().all(|k| rec.done.contains(k)));
    }

    #[test]
    fn borrowing() {
        let (mut t, mut rec) = (Tracker::default(), Record::default());
        start(&mut t, &mut rec, Mode::Classic, 1, BOTH, true);
        t.feat(Feat::Borrow { slot: 0, partner_left: 0 }, 30.0, &mut rec);
        t.feat(Feat::Lost { slot: 0, stunned: false }, 41.0, &mut rec);
        assert!(rec.done.contains("lifeline") && !rec.done.contains("borrowed_time"), "11 s is too long");
        t.feat(Feat::Borrow { slot: 0, partner_left: 0 }, 50.0, &mut rec);
        t.feat(Feat::Lost { slot: 0, stunned: false }, 59.0, &mut rec);
        assert!(rec.done.contains("borrowed_time"));
        start(&mut t, &mut rec, Mode::Classic, 2, BOTH, false);
        (0..3).for_each(|_| t.feat(Feat::Borrow { slot: 0, partner_left: 2 }, 0.0, &mut rec));
        assert!(rec.done.contains("loan_shark"), "5 in one run, across stages");
    }

    #[test]
    fn grenades_scores_and_stars() {
        let (mut t, mut rec) = (Tracker::default(), Record::default());
        start(&mut t, &mut rec, Mode::Classic, 1, SOLO, true);
        t.feat(Feat::Grenade { slot: 0, count: 5 }, 0.0, &mut rec);
        assert!(rec.done.contains("clean_sweep") && !rec.done.contains("six_pack"));
        t.feat(Feat::Grenade { slot: 0, count: 0 }, 0.0, &mut rec);
        t.feat(Feat::Grenade { slot: 0, count: 6 }, 0.0, &mut rec);
        t.feat(Feat::Score { slot: 0, score: 50_000 }, 0.0, &mut rec);
        t.feat(Feat::Star { slot: 0, level: 3 }, 0.0, &mut rec);
        (0..100).for_each(|_| t.feat(Feat::Cancel { slot: 0 }, 0.0, &mut rec));
        for k in ["overkill", "six_pack", "score_20k", "score_50k", "fully_loaded", "bullet_ballet"] {
            assert!(rec.done.contains(k), "{k}");
        }
        assert!(!rec.done.contains("score_100k"));
        start(&mut t, &mut rec, Mode::City, 1, SOLO, true);
        t.feat(Feat::Score { slot: 0, score: 200_000 }, 0.0, &mut rec);
        assert!(!rec.done.contains("score_100k"), "classic runs only");
    }
}
