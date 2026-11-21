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
    /// EASY MODE's own frontiers: easy clears move only these, so turning easy off opens nothing for free, and an easy
    /// run may start anything either frontier opened, so turning it on locks nothing.
    pub easy_unlocked: usize,
    pub easy_classic: usize,
}

impl Default for Progress {
    fn default() -> Self {
        Progress { unlocked: 1, classic: 1, easy_unlocked: 1, easy_classic: 1 }
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
        Progress {
            unlocked: get("unlocked=", Mode::City),
            classic: get("classic=", Mode::Classic),
            easy_unlocked: get("easy_unlocked=", Mode::City),
            easy_classic: get("easy_classic=", Mode::Classic),
        }
    }

    fn frontier(&mut self, mode: Mode, easy: bool) -> &mut usize {
        match (mode, easy) {
            (Mode::City, false) => &mut self.unlocked,
            (Mode::Classic, false) => &mut self.classic,
            (Mode::City, true) => &mut self.easy_unlocked,
            (Mode::Classic, true) => &mut self.easy_classic,
        }
    }

    /// Highest district / stage the players may start, in EASY MODE (`easy`) or not.
    pub fn open(&self, mode: Mode, easy: bool) -> usize {
        let (normal, easier) = match mode {
            Mode::City => (self.unlocked, self.easy_unlocked),
            Mode::Classic => (self.classic, self.easy_classic),
        };
        if easy { normal.max(easier) } else { normal }
    }

    /// District / stage `n` cleared: opens the next one, but only from the frontier: a clear past it (a LAN joiner
    /// playing the host's further stages, a smoke run) fills no gap. An `easy` clear moves only the easy frontier.
    /// True if that is news.
    pub fn clear(&mut self, mode: Mode, n: usize, easy: bool) -> bool {
        if n != self.open(mode, easy) || n >= cap(mode) {
            return false;
        }
        *self.frontier(mode, easy) = n + 1;
        true
    }

    /// The higher of both, per frontier. True if `self` grew.
    pub fn merge(&mut self, other: Progress) -> bool {
        let before = *self;
        self.unlocked = self.unlocked.max(other.unlocked);
        self.classic = self.classic.max(other.classic);
        self.easy_unlocked = self.easy_unlocked.max(other.easy_unlocked);
        self.easy_classic = self.easy_classic.max(other.easy_classic);
        *self != before
    }

    pub fn text(&self) -> String {
        format!(
            "unlocked={}\nclassic={}\neasy_unlocked={}\neasy_classic={}\n",
            self.unlocked, self.classic, self.easy_unlocked, self.easy_classic
        )
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
fn save_clear(
    session: Res<Session>,
    settings: Res<crate::settings::Settings>,
    mut progress: ResMut<Progress>,
    mut own: ResMut<OwnProgress>,
    mut pull: ResMut<super::cloud::Pull>,
) {
    let (mode, n, easy) = (session.mode, session.stage, settings.easy_mode);
    let news = progress.clear(mode, n, easy);
    let mine = match own.0.as_mut() {
        Some(own) => own.clear(mode, n, easy).then_some(*own),
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

    fn at(unlocked: usize, classic: usize) -> Progress {
        Progress { unlocked, classic, ..Progress::default() }
    }

    #[test]
    fn progress_parses_old_files_and_caps() {
        assert_eq!(Progress::parse(""), Progress::default());
        assert_eq!(Progress::parse("unlocked=4\n"), at(4, 1), "a file from before classic unlocks");
        let all = Progress::parse("unlocked=99\nclassic=999\n");
        assert_eq!((all.unlocked, all.classic), (CITY_DISTRICTS + 1, crate::levels::stage_count() + 1));
        let p = Progress { easy_classic: 9, ..at(3, 7) };
        assert_eq!(Progress::parse(&p.text()), p);
    }

    #[test]
    fn clears_open_the_next_one_from_the_frontier_only() {
        for mode in [Mode::City, Mode::Classic] {
            let mut p = at(3, 3);
            assert!(!p.clear(mode, 1, false), "replaying an old one changes nothing");
            assert!(!p.clear(mode, 5, false), "a clear past the frontier fills no gap");
            assert_eq!(p.open(mode, false), 3);
            assert!(p.clear(mode, 3, false));
            assert!(p.clear(mode, 4, false), "a chain from the frontier keeps going");
            assert_eq!(p.open(mode, false), 5);
            let last = cap(mode) - 1;
            *(if mode == Mode::City { &mut p.unlocked } else { &mut p.classic }) = last;
            assert!(p.clear(mode, last, false));
            assert_eq!(p.open(mode, false), cap(mode));
            assert!(!p.clear(mode, last + 1, false), "laps past the last map open nothing more");
        }
        let mut p = Progress::default();
        p.clear(Mode::Classic, 1, false);
        assert_eq!(p, at(1, 2), "modes are separate");
    }

    #[test]
    fn merge_takes_the_higher_per_mode() {
        let mut p = at(4, 2);
        assert!(p.merge(Progress { easy_unlocked: 7, ..at(2, 9) }));
        assert_eq!(p, Progress { easy_unlocked: 7, ..at(4, 9) });
        assert!(!p.merge(Progress::default()));
    }

    #[test]
    fn easy_mode_has_its_own_frontier() {
        let mut p = at(3, 3);
        assert_eq!(p.open(Mode::Classic, true), 3, "easy starts from the normal frontier");
        assert!(p.clear(Mode::Classic, 3, true));
        assert!(p.clear(Mode::Classic, 4, true));
        assert_eq!((p.open(Mode::Classic, false), p.open(Mode::Classic, true)), (3, 5), "easy clears open nothing normal");
        assert!(p.clear(Mode::Classic, 3, false));
        assert_eq!(p.open(Mode::Classic, true), 5);
        for _ in 4..7 {
            let n = p.open(Mode::Classic, false);
            p.clear(Mode::Classic, n, false);
        }
        assert_eq!(p.open(Mode::Classic, true), 7, "normal play past the easy frontier carries easy along");
    }
}
