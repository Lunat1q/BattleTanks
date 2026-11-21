# 0017 — Achievements: tracked by the game, mirrored to the host

Status: proposed (2026-11-20)

## Context
The owner wants achievements (list: `../achievements.md`, 55 entries) on the Android build through Google Play Games,
shown after a run and in a menu with what is still to get. The Windows exe gets none for now. Unlocks should follow
the player across devices, like City Run progress (ADR 0016).

## Decision
- **The game owns the state.** `achieve.rs` holds the list (key, counter steps, hidden flag; name and description for
  the menu) and a pure `Tracker` fed by `Feat` messages that the sim writes where things happen (kill, tank lost,
  stun, borrow, pickup, grenade count, shell cancel, base hit, secret opened, pause, stage won, run over). It only
  reads: nothing it does feeds back into the simulation, so LAN lockstep hashes are untouched. Only this device's
  slots count (couch: both; LAN: `net.me()`).
- **Saved locally** in `achievements.txt` (`done=<keys>`, `<key>=<steps>` for counters) and carried in the existing
  cloud copy next to `unlocked=N`: merge = union of done keys, `max` of each counter. Same rules as ADR 0016 (sync at
  start and after news, never write a cloud copy we couldn't read). Older builds keep only the `unlocked=` line; the
  next newer build that syncs puts the rest back, since its local copy is ahead.
- **Mirrored to the host** through `Platform::achieve: Option<fn(key: &str, steps: u32)>` (`steps` 0 = unlock, else
  the counter's total), called on a worker thread for each change, and once for everything at start (so a device that
  merged unlocks from the cloud tells the host too). On Android: Play Games `unlock` / `setSteps` (both idempotent, so
  resending is harmless). `None` (the exe) = no achievements at all: no tracking, no menu, no summary.
- **Shown in the game:** the run's new unlocks on the game over screen (classic) and on district results / game over
  (City Run); a title menu ACHIEVEMENTS (with the host hook) listing every entry: done ones with name and description,
  pending ones with their progress (`345 / 1000`), hidden pending ones as `???`, and `N / 55` on top.

## Alternatives
- Play Games as the only store (increment per event, load on start): counters would sum across devices, but a failed
  call loses steps, and the menu needs a load over the network before it can show anything.
- Separate cloud snapshot for achievements: no mixed-version caveat, but a second sync path for the same idea.

## Consequences
- Counters played on two devices at the same time merge by `max`, so one device's share can be lost. Fine for counters
  of 100-5000.
- Keys are permanent (Play can't delete a published achievement); a renamed key is a new achievement.
- Turning achievements on for the exe later = set `achieve` to a no-op in `Platform::default()`.
