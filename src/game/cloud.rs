//! City Run progress follows a player across their devices when the host has a cloud copy
//! (`Platform::cloud_load` / `cloud_save`, e.g. Play Games saved games on Android): synced once at start and after
//! every new unlock. Unlocks only grow, so merging is `max` and there is no conflict to resolve.

use super::city::Progress;
use bevy::prelude::*;
use std::sync::{Arc, Mutex};

/// A sync on its worker thread; its answer is the merged unlocks.
#[derive(Resource, Default)]
pub struct Pull(Option<Arc<Mutex<Option<usize>>>>);

impl Pull {
    /// Reads the cloud copy, merges `local` into it, writes it back if the cloud was behind.
    pub fn start(&mut self, platform: &crate::Platform, local: Progress) {
        let (Some(load), Some(save)) = (platform.cloud_load, platform.cloud_save) else { return };
        let slot = Arc::new(Mutex::new(None));
        let out = slot.clone();
        std::thread::spawn(move || {
            let merged = sync(load, save, local);
            if let Ok(mut o) = out.lock() {
                *o = merged;
            }
        });
        self.0 = Some(slot);
    }
}

/// `None` when the cloud can't be read (offline, not signed in): then nothing is written, so a copy we never saw
/// is never overwritten.
fn sync(load: fn() -> Result<String, String>, save: fn(&str) -> Result<String, String>, local: Progress) -> Option<usize> {
    let cloud = load().map_err(|e| info!("cloud save not read: {e}")).ok()?;
    let mut merged = Progress::parse(&cloud);
    if local.unlocked > merged.unlocked {
        merged.unlocked = local.unlocked;
        if let Err(e) = save(&merged.text()) {
            info!("cloud save not written: {e}");
        }
    }
    Some(merged.unlocked)
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Pull>()
        .add_systems(Startup, |platform: Res<crate::Platform>, progress: Res<Progress>, mut pull: ResMut<Pull>| {
            pull.start(&platform, *progress)
        })
        .add_systems(Update, apply);
}

/// Unlocks from another device land here. A LAN joiner's `Progress` holds the host's for the session, so it waits.
// ponytail: a pull answering mid-session on a joiner is dropped; the next start or clear syncs again.
fn apply(mut pull: ResMut<Pull>, mut progress: ResMut<Progress>, net: Res<crate::net::Net>) {
    let Some(merged) = pull.0.as_ref().and_then(|p| p.lock().ok()?.take()) else { return };
    pull.0 = None;
    if merged > progress.unlocked && net.me() != Some(1) {
        progress.unlocked = merged;
        progress.save_local();
    }
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
        let at = |n| Progress { unlocked: n };
        *CLOUD.lock().unwrap() = Some(String::new());
        assert_eq!(sync(load, save, at(3)), Some(3), "empty cloud takes the local unlocks");
        assert_eq!(cloud().as_deref(), Some("unlocked=3\n"));
        assert_eq!(sync(load, save, at(1)), Some(3), "a fresh device gets them back");
        assert_eq!(cloud().as_deref(), Some("unlocked=3\n"), "and doesn't push its lower count");
        *CLOUD.lock().unwrap() = None;
        assert_eq!(sync(load, save, at(5)), None);
        assert_eq!(cloud(), None, "unreadable cloud is left alone");
    }
}
