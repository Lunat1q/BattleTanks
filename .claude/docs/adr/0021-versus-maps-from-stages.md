# 0021 — VERSUS maps from the classic stages

Status: accepted (2026-11-23, owner). Supersedes ADR 0019's `versus_gen` generator and its RANDOM-at-runtime map.

## Context
ADR 0019 planned a brush generator (`versus_gen`) for ~12 pre-made VERSUS maps plus a fresh map per RANDOM round.
The owner asked instead: take the regular stage maps and mirror them.

## Decision
- A classic stage's bottom 13 rows (base, ring, steel cover) turned 180 degrees for the top half make a VERSUS map.
  Stage rows are already left-right mirrored, so this is the same point symmetry the hand-drawn maps have.
- `versus_map::sound` (reachability from player 1's spawn and the NPC corners, steel over each ring in the centre
  lane, no dead-end pockets) runs at startup over every stage half; those that pass join the 3 hand-drawn maps.
  At the time of writing 23 of 30 pass: 26 maps.
- RANDOM picks one of them each round with the gameplay dice (same list and dice on both LAN machines).
- No generator, no new map files.

## Consequences
- A stage map change can add or drop a VERSUS map, and shift the numbering after it; the test asks for at least
  8 stage halves so the pool can't quietly empty.
- Maps look like the classic stages they come from. If that gets dull, hand-drawn `levels/versus/` maps or a
  generator can still be added: both feed the same list.
