//! What the players may start: City Run districts and classic stages open one by one as they are cleared. Saved in
//! `city.txt` (the name predates classic unlocks) and in the host's cloud copy (`cloud`).

use super::*;
use crate::rules::CITY_DISTRICTS;

/// Highest district / classic stage the players may start (1-based); one past the last once all are cleared.
#[derive(Resource, Clone, Copy, PartialEq, Debug)]
pub struct Progress {
    /// City Run (`unlocked=` in the file, from before classic had unlocks).
    pub unlocked: usize,
    pub classic: usize,
}

impl Default for Progress {
    fn default() -> Self {
        Progress { unlocked: 1, classic: 1 }
    }
}

/// One past the last district / stage: everything cleared.
fn cap(mode: Mode) -> usize {
    match mode {
        Mode::City => CITY_DISTRICTS + 1,
        Mode::Classic => crate::levels::stage_count() + 1,
    }
}

impl Progress {
    pub fn parse(text: &str) -> Self {
        let get = |key: &str, mode| {
            let n = text.lines().find_map(|l| l.trim().strip_prefix(key)).and_then(|v| v.trim().parse().ok()).unwrap_or(1);
            usize::clamp(n, 1, cap(mode))
        };
        Progress { unlocked: get("unlocked=", Mode::City), classic: get("classic=", Mode::Classic) }
    }

    /// District / stage `n` cleared: opens the next one, but only from the frontier: a clear past it (a LAN joiner
    /// playing the host's further stages, a smoke run) fills no gap. True if that is news.
    pub fn clear(&mut self, mode: Mode, n: usize) -> bool {
        let at = match mode {
            Mode::City => &mut self.unlocked,
            Mode::Classic => &mut self.classic,
        };
        if n != *at || n >= cap(mode) {
            return false;
        }
        *at = n + 1;
        true
    }

    /// The higher of both, per mode. True if `self` grew.
    pub fn merge(&mut self, other: Progress) -> bool {
        let before = *self;
        self.unlocked = self.unlocked.max(other.unlocked);
        self.classic = self.classic.max(other.classic);
        *self != before
    }

    pub fn text(&self) -> String {
        format!("unlocked={}\nclassic={}\n", self.unlocked, self.classic)
    }

    /// Writes `city.txt` in the data folder.
    pub fn save_local(&self) {
        let Some(p) = crate::settings::data_file(SAVE) else { return };
        if let Err(e) = p.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&p, self.text())) {
            warn!("could not save progress to {}: {e}", p.display());
        }
    }
}

/// A LAN joiner's own progress while linked (`Progress` then holds the host's, which both maps and grids show).
/// `None` offline and on the host.
#[derive(Resource, Default)]
pub struct OwnProgress(pub Option<Progress>);

const SAVE: &str = "city.txt";

pub fn plugin(app: &mut App) {
    let progress = crate::settings::data_file(SAVE)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map_or_else(Progress::default, |t| Progress::parse(&t));
    app.insert_resource(progress)
        .init_resource::<OwnProgress>()
        .add_plugins(super::cloud::plugin)
        .add_systems(OnEnter(Screen::Results), save_clear);
}

/// Results = a stage or district cleared. Offline and on the host that moves `Progress` along and saves it. A LAN
/// joiner moves the host's copy along unsaved (both machines must agree on it) and their own by the same frontier rule.
fn save_clear(session: Res<Session>, mut progress: ResMut<Progress>, mut own: ResMut<OwnProgress>, mut pull: ResMut<super::cloud::Pull>) {
    let (mode, n) = (session.mode, session.stage);
    let news = progress.clear(mode, n);
    let mine = match own.0.as_mut() {
        Some(own) => own.clear(mode, n).then_some(*own),
        None => news.then_some(*progress),
    };
    if let Some(p) = mine {
        p.save_local();
        pull.want = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    impl Progress {
        fn open(&self, mode: Mode) -> usize {
            if mode == Mode::City { self.unlocked } else { self.classic }
        }
    }

    #[test]
    fn progress_parses_old_files_and_caps() {
        assert_eq!(Progress::parse(""), Progress::default());
        assert_eq!(Progress::parse("unlocked=4\n"), Progress { unlocked: 4, classic: 1 }, "a file from before classic unlocks");
        let all = Progress::parse("unlocked=99\nclassic=999\n");
        assert_eq!((all.unlocked, all.classic), (CITY_DISTRICTS + 1, crate::levels::stage_count() + 1));
        let p = Progress { unlocked: 3, classic: 7 };
        assert_eq!(Progress::parse(&p.text()), p);
    }

    #[test]
    fn clears_open_the_next_one_from_the_frontier_only() {
        for mode in [Mode::City, Mode::Classic] {
            let mut p = Progress { unlocked: 3, classic: 3 };
            assert!(!p.clear(mode, 1), "replaying an old one changes nothing");
            assert!(!p.clear(mode, 5), "a clear past the frontier fills no gap");
            assert_eq!(p.open(mode), 3);
            assert!(p.clear(mode, 3));
            assert!(p.clear(mode, 4), "a chain from the frontier keeps going");
            assert_eq!(p.open(mode), 5);
            let last = cap(mode) - 1;
            *(if mode == Mode::City { &mut p.unlocked } else { &mut p.classic }) = last;
            assert!(p.clear(mode, last));
            assert_eq!(p.open(mode), cap(mode));
            assert!(!p.clear(mode, last + 1), "laps past the last map open nothing more");
        }
        let mut p = Progress::default();
        p.clear(Mode::Classic, 1);
        assert_eq!(p, Progress { unlocked: 1, classic: 2 }, "modes are separate");
    }

    #[test]
    fn merge_takes_the_higher_per_mode() {
        let mut p = Progress { unlocked: 4, classic: 2 };
        assert!(p.merge(Progress { unlocked: 2, classic: 9 }));
        assert_eq!(p, Progress { unlocked: 4, classic: 9 });
        assert!(!p.merge(Progress::default()));
    }
}
