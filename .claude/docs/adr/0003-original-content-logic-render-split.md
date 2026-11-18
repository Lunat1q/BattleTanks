# 0003 — Original content; logic entities without sprites

Status: accepted (2026-11-18)

## Context
The game is "inspired by" the classic base-defence tank game: same genre and rules feel, but it must not be a
recreation of that specific game. A reference repo containing the original sprite sheets, the original 35 stage
maps and sounds was offered.

## Decision
1. Mechanics follow the genre (grid tanks, bricks/steel/water/trees/ice, base, power-up types). Stage maps,
   art, sounds, title and palette are our own. Assets and maps from existing games or rips are never imported.
2. `game/` spawns pure logic entities (components + `Transform`); `gfx/` attaches sprites on `Added<T>` and
   reacts to `Fx`/`Sfx` messages. Gameplay code never touches rendering types.

## Alternatives
- Loading the classic maps/sprites from the reference repo: rejected (not ours to ship, and the point is a
  better-looking original).
- Sprites spawned by gameplay systems: simpler at first, but couples rules to art and makes logic harder to test.

## Consequences
- New stages are hand-made in `levels/` (mirrored half-rows make symmetric maps quick).
- A new visual for an entity = one attach system in `gfx/world.rs`; rules stay untouched.
