# 0005 — City Run shares the stage; terrain is tall, a View scrolls

Status: accepted (2026-11-18)

## Context
City Run is a second mode: ten districts, each 15+ screens tall, scrolling as the players push up, with turrets,
gates, mini-bosses and a boss. It needs the same tanks, bullets, terrain, power-ups, HUD and pause flow as a
classic stage.

## Decision
1. One `Screen::Stage` for both modes; `Session.mode` picks the setup (`levels` vs `city_map`) and a few rules
   (no base, no shovel, win = boss down). City-only systems live in `game/city.rs`, run in `Sim`, and only while
   the `CityRun` resource exists.
2. `Terrain` keeps its 52-sub-cell width but has any number of rows. A `View { bottom }` resource is the visible
   208-unit slice: tanks can't leave it, bullets pop at its edges, the camera follows it. Classic stages keep
   `bottom = 0`, so they behave exactly as before.
3. Bosses and turrets are ordinary enemy tanks with a `Rank` (size, HP, points, fire rate); a turret is a tank with
   speed 0. Collision, hits, sprites and AI reuse the existing paths.

## Alternatives
- Separate `Screen::City` with its own systems: duplicates the stage lifecycle, pause, banner and HUD.
- Streaming terrain chunks in and out: unnecessary at these sizes (~20k sub-cell sprites at most).
- Separate boss/turret entity types: every system that touches tanks would need a second query.

## Consequences
- Anything placed at `FIELD` coordinates must add `View.bottom` in City Run (respawn, power-up drops).
- Maps must keep a full-width fortress wall per bastion; `city_map` tests check every gate actually bars the way.
