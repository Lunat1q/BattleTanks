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
- [x] Settings screen: FPS limit (default 60), VSync, fullscreen, volume, screen shake, FPS counter, friendly fire, vibration; saved in `%APPDATA%`
- [x] Version shown on the title; GitHub release v0.1.0 with the exe

### M3 — Looks & sound ✅ (first pass)
- [x] SDF painter; procedural tanks (8 models, tread animation), bricks, steel, water (3 frames), trees, ice, ground
- [x] HDR bloom bullets, muzzle flashes, explosions (flash/fire/smoke/sparks), brick debris, screen shake
- [x] Spawn sparkle, shield bubble, armor hit flash, bonus-carrier red flash, freeze tint, score pop-ups
- [x] Synthesised sound effects + short jingles (no asset files)

### M3.5 — Content & identity ✅
- [x] 30 stages (22 new: 09–30), rosters ramping to heavy armour; reachability test guards every map
- [x] App icon: exe resource (Explorer/Steam) + window/taskbar icon, painted from the game's own tank art
- [x] Version 0.2.0 (unreleased build in the owner's game folder)

### M5 — City Run ✅ (first pass)
- [x] Tall terrain + scrolling `View` (follow lead, keep trailing tank, never back)
- [x] 10 districts (15–24 screens), bastions with turrets + gates, mini-bosses, boss; tests for winnability and gates
- [x] District map screen (pixel-art city backdrop), unlocks saved in `%APPDATA%`; power-ups/lives reset per district
- [x] Districts redrawn by theme (`city_gen`): each looks like its name, no street stretch repeats (owner: copies bothered)
- [ ] Couch-test difficulty and length; boss HP bar if bosses feel like sponges

### M4 — Ship to the TV ⏳
- [x] Release profile (thin LTO, stripped, no console window)
- [ ] Build release exe on the gaming PC, add to Steam (see README), test through Sunshine/Moonlight with 2 pads
- [ ] Couch tuning pass: enemy speeds, spawn interval, max enemies in 2P, AI aggression (`rules.rs`, `enemy.rs`)
- [ ] Check Steam Input doesn't double controllers (disable per-game Steam Input if it does)

### M7 — Couch feedback (v0.4.0) ✅
- [x] Music normalised to −16 LUFS; 5 new tracks (fast 2–4, hardfight 2, bossfight 2); pools rotate when a track ends
- [x] Borrow a spare tank from the partner when out (both modes); HUD hint
- [x] Scouts slower (66 → 54) and less twitchy (longer turn gap)
- [x] City Run: +1 spare tank at district start; brick stashes with a star pair and an extra-tank pair per district
- [x] Setting KEEP POWER (stars survive losing a tank)
- [x] README screenshots (`docs/screenshots`, smoke hook)

### Next ideas (not started — confirm before building)
- [x] Persist hi-score (`scores.txt` in `%APPDATA%`) + world board: time bonus, stage log sent at game over to the
  owner's score server (`score_server` submodule of the Android repo), name entry, rank on the game over screen (ADR 0009)
- [x] Steam grid artwork: capsules, hero, logo, icon in `docs/steam/` (`write_steam_art`), README steps
- [x] Background music: owner's 11 tracks, embedded Ogg Vorbis, per screen / stage tier / boss (ADR 0007)
- [x] Results-screen count-up animation like the classic tally (blip per kill, A skips)
- [x] City Run world boards: district clear time + kills, one fastest-first board per district and player count,
  server checks time and kills against the district (ADR 0010)
- [ ] World board live check: server reachable at `scores.maxpodolski.com` (tunnel route + DNS added, key and rules checked live); first real run from the couch still to come

## Current state

City Run added, released as v0.3.0 on GitHub: 10 districts, district map, smoke screenshots checked for the map, a district start, a
bastion and a boss arena. Not yet played on the couch.

M1–M3 done, plus Settings (FPS limit — owner saw ~1000 fps — volume, fullscreen, shake, FPS counter); all checks green (fmt, clippy, 30 tests, released as v0.1.0,
release build). Limiter measured at 59 fps (60 cap) and 140 fps (144 cap) in smoke runs. Smoke screenshots verified for the title
and stages 1, 2, 4, 5. Not yet played with real controllers — that's the first M4 step.

## Blockers

- M4 couch steps need the owner with the TV setup and real pads.
