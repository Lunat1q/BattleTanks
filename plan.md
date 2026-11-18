# Plan & progress

Living checklist. Update it in the same change that moves an item. A new session resumes from here.

## Goal

A polished two-player couch tank game for the TV: launch from Steam (non-Steam shortcut) through
Sunshine/Vibepollo → Moonlight, play with two controllers, classic rules, nicer looks. One exe, no installer.

## Milestones

### M1 — Playable core ✅
- [x] Bevy 0.20 project, fullscreen borderless window, hidden cursor, `--windowed`, F11 / Alt+Enter toggle
- [x] Field geometry, destructible terrain (brick 4×4 chips, steel, water, trees, ice), base ring
- [x] Stage file format (mirrored half rows), 8 original stages, looping with harder rosters
- [x] Player tanks: grid-snapped turns, ice slide, star levels 0–3, respawn with shield, lives, extra life per 20k
- [x] Enemies: 4 types, 20 per stage, 3 spawn points, max 4 (1P) / 6 (2P) on field, bonus carriers
- [x] Enemy AI: wander biased to the base, hunt players sometimes, turn/shoot when blocked
- [x] Bullets: sub-stepped, brick/steel rules, bullet-vs-bullet cancel, friendly fire paralyzes
- [x] Power-ups: helmet, clock, shovel (with blink-back), star, grenade, tank
- [x] Win/lose flow: intro banner → play → clear/lost → results / game over → title

### M2 — Input & TV ✅
- [x] Device abstraction: pads + two keyboard layouts; join on title with A/Space/Enter, B leaves
- [x] Pad navigation everywhere (stick edge + D-pad), Start pause, pause menu with resume/title/exit
- [x] Controller disconnect auto-pauses; any free pad pressing A takes over the slot
- [x] HUD side panels sized in `vh` for 16:9 TVs
- [x] Settings screen: FPS limit (default 60), VSync, fullscreen, volume, screen shake, FPS counter; saved in `%APPDATA%`

### M3 — Looks & sound ✅ (first pass)
- [x] SDF painter; procedural tanks (8 models, tread animation), bricks, steel, water (3 frames), trees, ice, ground
- [x] HDR bloom bullets, muzzle flashes, explosions (flash/fire/smoke/sparks), brick debris, screen shake
- [x] Spawn sparkle, shield bubble, armor hit flash, bonus-carrier red flash, freeze tint, score pop-ups
- [x] Synthesised sound effects + short jingles (no asset files)

### M4 — Ship to the TV ⏳
- [x] Release profile (thin LTO, stripped, no console window)
- [ ] Build release exe on the gaming PC, add to Steam (see README), test through Sunshine/Moonlight with 2 pads
- [ ] Couch tuning pass: enemy speeds, spawn interval, max enemies in 2P, AI aggression (`rules.rs`, `enemy.rs`)
- [ ] Check Steam Input doesn't double controllers (disable per-game Steam Input if it does)

### Next ideas (not started — confirm before building)
- [ ] Persist hi-score (small file in `%APPDATA%`)
- [ ] More stages (format is in `levels.rs`; add `levels/NN.txt` + one `include_str!`)
- [ ] Exe icon + Steam grid artwork
- [ ] Background music loop
- [ ] Results-screen count-up animation like the classic tally

## Current state

M1–M3 done, plus Settings (FPS limit — owner saw ~1000 fps — volume, fullscreen, shake, FPS counter); all checks green (fmt, clippy, 30 tests,
release build). Limiter measured at 59 fps (60 cap) and 140 fps (144 cap) in smoke runs. Smoke screenshots verified for the title
and stages 1, 2, 4, 5. Not yet played with real controllers — that's the first M4 step.

## Blockers

None.
