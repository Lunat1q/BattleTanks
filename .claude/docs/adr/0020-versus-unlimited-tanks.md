# 0020 — VERSUS: unlimited tanks, a growing respawn wait

Status: accepted (2026-11-23, owner). Supersedes ADR 0019's "3 tanks each" and its knock-out win.

## Context
ADR 0019 gave each VERSUS player 3 tanks (EASY MODE 2 more) and let a round end when a player ran out. The owner
would rather have no tank limit and make each loss cost more time instead.

## Decision
- Tanks are unlimited in VERSUS. The n-th tank lost in a round waits before it comes back beside its base:
  1, 3, 5, 7, 10, 15 s, then 5 s more each time, at most 30 s (`rules::versus_respawn`; the 1 s spawn twinkle is
  part of the wait). The count starts again every round.
- A round ends only when a base falls (both on one tick: a draw).
- The wait reuses the slot's `out` / `out_wait` (already in the LAN snapshot); `Slot.deaths` counts the losses.
  `game::versus::respawn` brings the tank back when the wait is over. EASY MODE changes nothing here.
- HUD: `LOST n` instead of the spare count, `BACK IN n` while waiting.

## Consequences
- A defender can always come back, so a lead in kills buys time on the other base, not the round itself.
- The 30 s cap keeps a long round from turning into one player alone; tune it in `rules.rs` after couch play.
