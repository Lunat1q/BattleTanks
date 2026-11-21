//! Progress (City Run districts, classic stages) and achievements follow a player across their devices when the host
//! has a cloud copy (`Platform::cloud_load` / `cloud_save`, e.g. Play Games saved games on Android): synced once at
//! start, after every new unlock and after new achievements. Both only grow, so merging is `max` / union and there is no
//! conflict to resolve. The copy is `city.txt`'s text followed by `achievements.txt`'s.

use super::progress::{OwnProgress, Progress};
use crate::achieve::Record;
use bevy::prelude::*;
use std::sync::{Arc, Mutex};

type Answer = Arc<Mutex<Option<(Progress, Record)>>>;

/// One sync at a time on a worker thread; its answer is the merged copy. Set `want` to ask for one.
#[derive(Resource)]
pub struct Pull {
    busy: Option<Answer>,
    pub want: bool,
}

impl Default for Pull {
    fn default() -> Self {
        Pull { busy: None, want: true }
    }
}

/// `None` when the cloud can't be read (offline, not signed in): then nothing is written, so a copy we never saw
/// is never overwritten.
fn sync(
    load: fn() -> Result<String, String>,
    save: fn(&str) -> Result<String, String>,
    local: Progress,
    rec: &Record,
) -> Option<(Progress, Record)> {
    let cloud = load().map_err(|e| info!("cloud save not read: {e}")).ok()?;
    let mut merged = Progress::parse(&cloud);
    let mut won = Record::parse(&cloud);
    let behind = merged.merge(local);
    if (won.merge(rec) || behind)
        && let Err(e) = save(&(merged.text() + &won.text()))
    {
        info!("cloud save not written: {e}");
    }
    Some((merged, won))
}

pub fn plugin(app: &mut App) {
    // The achievements plugin replaces it with the saved record; headless tests run without one.
    app.init_resource::<Record>().init_resource::<Pull>().add_systems(Update, (kick, apply).chain());
}

/// A LAN joiner syncs their own progress, not the host's they play on.
fn kick(mut pull: ResMut<Pull>, platform: Res<crate::Platform>, progress: Res<Progress>, own: Res<OwnProgress>, rec: Res<Record>) {
    if !pull.want || pull.busy.is_some() {
        return;
    }
    pull.want = false;
    let (Some(load), Some(save)) = (platform.cloud_load, platform.cloud_save) else { return };
    let slot: Answer = Arc::new(Mutex::new(None));
    let out = slot.clone();
    let (local, rec) = (own.0.unwrap_or(*progress), rec.clone());
    std::thread::spawn(move || {
        let merged = sync(load, save, local, &rec);
        if let Ok(mut o) = out.lock() {
            *o = merged;
        }
    });
    pull.busy = Some(slot);
}

/// Unlocks from another device land here; on a LAN joiner in their own copy (the host's stays as both machines know it).
fn apply(mut pull: ResMut<Pull>, mut progress: ResMut<Progress>, mut own: ResMut<OwnProgress>, mut rec: ResMut<Record>) {
    let Some(busy) = &pull.busy else { return };
    if Arc::strong_count(busy) > 1 {
        return; // still running
    }
    let answer = busy.lock().ok().and_then(|mut o| o.take());
    pull.busy = None;
    let Some((merged, won)) = answer else { return };
    let mine = own.0.as_mut().unwrap_or(&mut *progress);
    if mine.merge(merged) {
        mine.save_local();
    }
    // Saved and told to the host by `achieve::mirror`.
    rec.merge(&won);
}

#[cfg(test)]
mod tests {
    use super::*;

    static CLOUD: Mutex<Option<String>> = Mutex::new(None);

    fn load() -> Result<String, String> {
        CLOUD.lock().unwrap().clone().ok_or_else(|| "offline".into())
    }
    fn save(text: &str) -> Result<String, String> {
        *CLOUD.lock().unwrap() = Some(text.into());
        Ok(String::new())
    }
    fn cloud() -> Option<String> {
        CLOUD.lock().unwrap().clone()
    }

    #[test]
    fn sync_merges_upward_and_never_writes_blind() {
        let at = |n| Progress { unlocked: n, ..Progress::default() };
        let none = Record::default();
        *CLOUD.lock().unwrap() = Some(String::new());
        assert_eq!(sync(load, save, at(3), &none).map(|m| m.0.unlocked), Some(3), "empty cloud takes the local unlocks");
        assert_eq!(cloud().as_deref(), Some("unlocked=3\nclassic=1\neasy_unlocked=1\neasy_classic=1\n"));
        assert_eq!(sync(load, save, at(1), &none).map(|m| m.0.unlocked), Some(3), "a fresh device gets them back");
        assert_eq!(
            cloud().as_deref(),
            Some("unlocked=3\nclassic=1\neasy_unlocked=1\neasy_classic=1\n"),
            "and doesn't push its lower count"
        );
        let mut rec = Record::default();
        rec.unlock("digger");
        rec.add("kills_100", 7);
        let (_, merged) = sync(load, save, at(1), &rec).unwrap();
        assert_eq!(
            cloud().as_deref(),
            Some("unlocked=3\nclassic=1\neasy_unlocked=1\neasy_classic=1\ndone=digger\nkills_100=7\n"),
            "achievements ride along"
        );
        assert!(merged.done.contains("digger"));
        *CLOUD.lock().unwrap() = Some("unlocked=2\n".into());
        let (n, merged) = sync(load, save, at(1), &none).unwrap();
        assert_eq!((n.unlocked, n.classic, merged.done.len()), (2, 1, 0), "an older build's copy (no achievement lines) reads fine");
        *CLOUD.lock().unwrap() = None;
        assert!(sync(load, save, at(5), &rec).is_none());
        assert_eq!(cloud(), None, "unreadable cloud is left alone");
    }
}
