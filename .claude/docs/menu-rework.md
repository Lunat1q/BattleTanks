# Title menu rework (built in 0.7.0, ADR 0018)

Owner decisions of 2026-11-21, kept for the record; `domain.md` and ADR 0018 describe what shipped. Changes made while
building: the cards say PLAYER 1 / PLAYER 2 (owner), and the LAN ping line stays under the hi score.

## Goal

Fewer title entries, join state shown as player cards, Classic stage picked on a tile grid where stages open one by
one as they are cleared.

## Title screen

```
                 <tank  BATTLE TANKS  tank>

 +----------+        > CLASSIC            +----------+
 |  [tank]  |          CITY RUN           |  [tank]  |
 |    1P    |          LAN CO-OP          |    2P    |
 |CONTROLLER|          HI SCORES          | PRESS A  |
 |  READY   |          ACHIEVEMENTS       | TO JOIN  |
 | B: leave |          SETTINGS           | (blinks) |
 +----------+          EXIT               +----------+

                       HI SCORE 12400
               (controls hint for this host only)
```

- **Removed:** the "two-player tank defence" tagline, the "PRESS A TO JOIN" / START menu line, the `STAGE < n >`
  line, and the bottom slot line (`SlotLine`). With 7 items at most, the `n > 7` tighter packing goes too.
- **Menu** (`ui/title.rs` `items`): `Classic, City, [Lan], HiScores, [Achievements], Settings, Exit`. When linked:
  `Classic, City, Leave`. EXIT stays on every host, Android too (one press beats a swipe).
- CLASSIC and CITY RUN are dimmed while nobody has joined, and A on them does nothing. Joining works as now: A
  (Space / Enter) from a device that has not joined takes a free slot, and B leaves it.
- **Player cards:** 1P sits left of the menu and 2P right, both centred vertically, with a border in the player's
  colour (GOLD / GREEN).
  - Empty card: dim tank and a blinking "PRESS A TO JOIN".
  - Empty card while a connected pad has not joined ("controller found"): it names the pad and what to press, e.g.
    "CONTROLLER - PRESS A". Keyboard: "SPACE" / "ENTER".
  - Joined card: bright tank, device name, READY, "B leave".
  - Linked: the cards carry the two machines' names, and the ping line moves into them.
- **Controls hint:** only what this host has, not today's mixed pad + keys line. The pad line is always shown, the
  keys line only on hosts with a keyboard. That needs a generic `Platform` knob; touch hosts (phones) get their own
  line.

## Classic stage select (new `Screen::StageSelect`, `ui/stages.rs`)

- CLASSIC → a grid of tiles, 6 × 5 for 30 stages. Columns come from a constant; rows follow `levels::stage_count()`.
- Up, down, left and right move a pulsing outline (city map style). A starts, B goes back to the title.
- The cursor can't move onto a locked tile; a move that would land on one does nothing. Stages open in order, so
  the open tiles are always 1..=N.
- The cursor starts on the highest open stage (one A to continue).
- Tile looks: cleared = gold frame, next unbeaten = bright frame, locked = dark with a lock mark. Each tile shows
  the stage number. No scores on tiles (too much screen space).
- **Mini-map on each tile:** one pixel per terrain tile from `levels::load(n).terrain`, coloured by tile kind and
  drawn with nearest sampling. Painted once at startup (30 small images, cheap); dimmed on locked tiles.
- `TitleMenu.stage` goes away; the grid's selection is its own resource. `net/mod.rs` resets it on link.
- The smoke hook (`BT_STAGE`) skips the lock.
- The City Run district map stays exactly as it is (the owner likes the path look).

## Classic unlocks

- `game::city::Progress` gets `classic: usize` next to `unlocked`. It's saved as a `classic=N` line in the same
  `city.txt`, so the cloud copy carries it. A missing line means 1.
- No migration. The game is in closed test, so everyone starts at stage 1.
- On `OnEnter(Results)` with `Mode::Classic`, a clear of stage n opens n + 1, capped at `stage_count()`. Past the
  last map the maps loop, but there are no more tiles.
- Cloud sync merges with `max` on both fields; it still only grows.

## LAN co-op unlocks (both modes)

- During a link the host's progress applies: the grid and the city map show the host's opens. The joiner's own
  progress is kept aside, as today (`Lockstep.restore`).
- **Host:** every clear counts, as now.
- **Joiner:** a clear of stage / district n, made together while linked, counts for the joiner only if the
  joiner's own progress already reached n (`n <= own`). Then the joiner's own n + 1 opens. A joiner never gets a gap
  filled. Example: their own open is 3, they clear 5 with the host, and nothing opens for them (3 and 4 stay
  unbeaten). A chain from their own frontier keeps counting: own 5, clear 5, 6, 7 in a row → own 8.
- The joiner saves their own progress (`city.txt` + cloud), never the host's. When the link ends, the joiner's
  updated own progress comes back instead of the stale `restore` copy.
- Tests: parsing an old `city.txt` gives `classic == 1`; a clear opens n + 1 up to the cap; joiner rule (frontier,
  gap, chain) for both modes; grid navigation as a pure `step(sel, dir, open) -> sel` (no move onto a locked tile,
  edges); cloud merge of both fields.

## Not doing

- Scores or stars on tiles (screen space).
- Hiding EXIT on Android.
- Restyling the City Run map.
- Merging HI SCORES and ACHIEVEMENTS (not asked; possible later if the menu feels long again).
