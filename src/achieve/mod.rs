//! Achievements (ADR 0017): the game keeps the record (`achievements.txt`, carried in the cloud copy next to the
//! City Run unlocks) and mirrors it to the host (`Platform::achieve`, Play Games on Android). A host without the
//! hook gets none: no tracking, no menu, no run summary.

pub mod list;
mod tracker;

pub use list::{Ach, LIST};
pub use tracker::Feat;
use tracker::Tracker;

use crate::game::{Mode, Screen};
use bevy::prelude::*;
use std::collections::{BTreeMap, BTreeSet};

const SAVE: &str = "achievements.txt";

/// What this device has earned.
#[derive(Resource, Default, Clone, Debug)]
pub struct Record {
    pub done: BTreeSet<&'static str>,
    /// Counter progress (counters only).
    pub steps: BTreeMap<&'static str, u32>,
    /// Unlocked in the current run (classic) or district try (City Run), for the summary.
    pub news: Vec<&'static str>,
    /// Changed since the host last heard.
    dirty: BTreeSet<&'static str>,
    /// Unlocked here since the last cloud sync.
    fresh: bool,
}

impl Record {
    /// True if `key` is news.
    pub fn unlock(&mut self, key: &str) -> bool {
        let Some(a) = list::find(key) else {
            debug_assert!(false, "no achievement {key}");
            return false;
        };
        if !self.done.insert(a.key) {
            return false;
        }
        self.news.push(a.key);
        self.dirty.insert(a.key);
        self.fresh = true;
        true
    }

    /// Counts `n` towards a counter; unlocks it at its target.
    pub fn add(&mut self, key: &str, n: u32) {
        let Some(a) = list::find(key).filter(|a| a.steps > 0) else {
            debug_assert!(false, "no counter {key}");
            return;
        };
        if self.done.contains(a.key) {
            return;
        }
        let s = self.steps.entry(a.key).or_default();
        *s = (*s + n).min(a.steps);
        self.dirty.insert(a.key);
        if *s >= a.steps {
            self.unlock(a.key);
        }
    }

    pub fn steps_of(&self, a: &Ach) -> u32 {
        if self.done.contains(a.key) { a.steps } else { self.steps.get(a.key).copied().unwrap_or(0) }
    }

    /// Takes in another copy's unlocks and progress (union / max). True if anything here changed.
    pub fn merge(&mut self, other: &Record) -> bool {
        let mut changed = false;
        for &k in &other.done {
            if self.done.insert(k) {
                self.dirty.insert(k);
                changed = true;
            }
        }
        for (&k, &n) in &other.steps {
            let s = self.steps.entry(k).or_default();
            if n > *s {
                *s = n;
                self.dirty.insert(k);
                changed = true;
            }
        }
        changed
    }

    /// `done=a,b` and `<counter>=<steps>` lines; lines it doesn't know (`unlocked=` in the cloud copy) are skipped.
    pub fn parse(text: &str) -> Record {
        let mut r = Record::default();
        for (k, v) in text.lines().filter_map(|l| l.trim().split_once('=')) {
            if k == "done" {
                r.done.extend(v.split(',').filter_map(|k| list::find(k.trim())).map(|a| a.key));
            } else if let (Some(a), Ok(n)) = (list::find(k).filter(|a| a.steps > 0), v.parse::<u32>()) {
                r.steps.insert(a.key, n.min(a.steps));
            }
        }
        r
    }

    pub fn text(&self) -> String {
        let mut out = String::new();
        if !self.done.is_empty() {
            out += &format!("done={}\n", self.done.iter().copied().collect::<Vec<_>>().join(","));
        }
        for (k, n) in self.steps.iter().filter(|&(_, &n)| n > 0) {
            out += &format!("{k}={n}\n");
        }
        out
    }

    /// The host calls for what changed: `(key, 0)` unlocks, `(key, steps)` sets a counter.
    fn take_dirty(&mut self) -> Vec<(&'static str, u32)> {
        let keys = std::mem::take(&mut self.dirty);
        keys.into_iter()
            .filter_map(list::find)
            .map(|a| (a.key, if a.steps == 0 { 0 } else { self.steps_of(a) }))
            .filter(|&(k, n)| if n == 0 { self.done.contains(k) } else { true })
            .collect()
    }

    fn save_local(&self) {
        let Some(p) = crate::settings::data_file(SAVE) else { return };
        if let Err(e) = p.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&p, self.text())) {
            warn!("could not save achievements to {}: {e}", p.display());
        }
    }
}

#[derive(Resource, Default)]
struct Seat(Tracker);

/// The host has achievements.
pub fn on(platform: Res<crate::Platform>) -> bool {
    platform.achieve.is_some()
}

pub fn plugin(app: &mut App) {
    let rec =
        crate::settings::data_file(SAVE).and_then(|p| std::fs::read_to_string(p).ok()).map_or_else(Record::default, |t| Record::parse(&t));
    app.insert_resource(rec)
        .init_resource::<Seat>()
        .add_systems(Startup, tell_all.run_if(on))
        .add_systems(OnEnter(Screen::Title), |mut rec: ResMut<Record>| rec.news.clear())
        .add_systems(Update, (track.after(crate::game::Sim), mirror.run_if(not(in_state(Screen::Stage)))).chain().run_if(on));
}

/// Once at start, so a host that missed something (another device's unlocks merged in) catches up.
fn tell_all(mut rec: ResMut<Record>) {
    let keys: Vec<&'static str> = rec.done.iter().chain(rec.steps.keys()).copied().collect();
    rec.dirty.extend(keys);
}

fn track(
    mut feats: MessageReader<Feat>,
    mut seat: ResMut<Seat>,
    mut rec: ResMut<Record>,
    run: Option<Res<crate::game::StageRun>>,
    net: Res<crate::net::Net>,
    settings: Res<crate::settings::Settings>,
) {
    let clock = run.map_or(0.0, |r| r.clock);
    seat.0.seat(net.me().map(usize::from));
    for &f in feats.read() {
        if let Feat::Start { mode, fresh, .. } = f
            && (fresh || mode == Mode::City)
        {
            rec.news.clear();
        }
        // EASY MODE runs earn nothing.
        if !settings.easy_mode {
            seat.0.feat(f, clock, &mut rec);
        }
    }
}

/// Off the stage: save what changed, tell the host on a worker thread, and sync the cloud copy after an unlock.
fn mirror(mut rec: ResMut<Record>, platform: Res<crate::Platform>, mut pull: ResMut<crate::game::cloud::Pull>) {
    if rec.dirty.is_empty() {
        return;
    }
    rec.save_local();
    if std::mem::take(&mut rec.fresh) {
        pull.want = true;
    }
    let calls = rec.take_dirty();
    if let Some(achieve) = platform.achieve {
        std::thread::spawn(move || calls.into_iter().for_each(|(k, n)| achieve(k, n)));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_round_trips_and_merges_upward() {
        let mut a = Record::default();
        a.unlock("first_blood");
        a.add("kills_1000", 345);
        a.add("boss_slayer", 12);
        assert!(a.done.contains("boss_slayer"), "a counter at its target unlocks");
        let text = a.text();
        assert_eq!(text, "done=boss_slayer,first_blood\nboss_slayer=10\nkills_1000=345\n");
        let b = Record::parse(&format!("unlocked=4\n{text}nonsense=3\ndone=gone_key,stage_1\n"));
        assert_eq!((b.done.len(), b.steps["kills_1000"]), (3, 345), "unknown keys and lines are skipped");
        let mut c = Record::parse("kills_1000=900\n");
        assert!(c.merge(&b));
        assert_eq!((c.steps["kills_1000"], c.done.len()), (900, 3), "max of counters, union of unlocks");
        assert!(!c.clone().merge(&b), "nothing new");
        assert!(c.news.is_empty(), "merged unlocks aren't this run's news");
    }

    #[test]
    fn the_host_hears_unlocks_and_counter_totals() {
        let mut r = Record::default();
        r.add("kills_100", 3);
        r.unlock("own_goal");
        assert_eq!(r.take_dirty(), vec![("kills_100", 3), ("own_goal", 0)]);
        assert!(r.take_dirty().is_empty());
        r.add("kills_100", 200);
        assert_eq!(r.take_dirty(), vec![("kills_100", 100)]);
    }
}
