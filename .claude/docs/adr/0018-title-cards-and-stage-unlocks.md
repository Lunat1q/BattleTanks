# 0018 — Title with player cards, classic stages unlock in order

Status: accepted (2026-11-21, built in 0.7.0)

## Context
The title had grown to eight lines: a "PRESS A TO JOIN / START" line that repeated the slot line under the menu, a
`STAGE < n >` number picker that let anyone start any stage, and the modes, boards and settings. The owner wanted a
shorter menu, the join state shown as player cards, and classic stages picked on tiles that open one by one, like
City Run districts.

## Decision
- **Title:** the logo is drawn from the game's own brick ("BATTLE") and steel ("TANKS") art instead of a font, and
  each player's tank drives in beside it when they join. No tagline and no START or STAGE line. The menu is CLASSIC, CITY RUN, LAN CO-OP, HI SCORES,
  ACHIEVEMENTS, SETTINGS, EXIT; linked over LAN it is CLASSIC, CITY RUN, LEAVE LAN. EXIT stays on every host.
  - The modes are dimmed until someone joins.
  - Join cards sit left (1P) and right (2P) of the menu. An empty card says what to press, and names a pad that is
    on but in no slot ("CONTROLLER FOUND").
  - The control hints show only this host's inputs. `Platform::keyboard` (default on) adds the keys line.
- **CLASSIC → `Screen::StageSelect`:** a 6 × 5 grid of tiles, each with a one-pixel-per-sub-cell mini-map
  (`gfx::art_map::stage_map`, painted once at startup). The cursor moves only over open tiles: left/right step
  through the stages in order, up/down jump a row.
- **Classic progress** lives next to City Run's in `Progress` (`game/progress.rs`), saved as a `classic=N` line in
  the same `city.txt` and the same cloud copy. Files without the line read as 1. There is no migration: the app is
  in closed test, so everyone starts at stage 1.
- **One frontier rule for both modes:** clearing stage / district `n` opens `n + 1` only when `n` is the highest
  open one. Replays change nothing, and a clear past the frontier fills no gap.
- **LAN:** the host's progress applies to the session (it travels in `Hello`; `PROTO` 3). The joiner's own copy
  waits in `OwnProgress`, moves by the same rule on each clear made together, is saved and synced to the joiner's
  cloud, and comes back when the link ends. So a joiner whose frontier is 3 opens nothing by clearing stage 5 with
  the host, but one at 5 who clears 5, 6 and 7 ends at 8.

## Consequences
- Old saves load unchanged. A build from before 0.7.0 drops the `classic=` line when it rewrites the cloud copy;
  the next 0.7.0 sync puts it back, since local only grows.
- Smoke runs (`BT_STAGE`) still start any stage, but such a clear opens nothing.
- Before this, a LAN joiner never saved unlocks; now they save their own, never the host's.
