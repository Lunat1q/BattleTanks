# 0010 — City Run on the world board: clear time per district, checked against the map

Status: accepted (2026-11-18)

## Context
City Run should be on the world board too, ranked by speed rather than points, and the server should be able to tell
a real clear from a made-up one. A time can't be recomputed like a points total.

## Decision
- After each district clear the players enter names as at a classic game over (once the tally is counted), and the
  game sends the district, the play time in tenths (banner and pauses excluded, stops at the boss kill) and the kills
  per player: ordinary tanks by type plus turrets, mini-bosses and the boss (`Slot::big`, counted in `combat`).
- The server keeps one fastest-first board per district and player count (its ADR 0003) and checks the clear against
  a per-district table: climb time (straight drive at full speed from the start until the boss is in view), placed
  enemies, turrets, mini-bosses. Every turret and mini-boss has to die on the way, the boss once, and ordinary kills
  can't exceed the placed tanks plus one reinforcement per interval.
- The table lives on both sides (`scores.rs` `CITY_FACTS`, server `CityRules.Facts`); the game test computes it from
  the maps and fails on any map change.

## Alternatives
- Sending the whole district path or an input log: much stronger, but big bodies and a replay engine on the server.
- The server parsing the map files: exact, but couples the server build to this repo.

## Consequences
- Changing a district map means updating the server table in the same release, or real clears of it are refused.
- A careful cheat can still post a plausible time above the physical floor (as with ADR 0009).
