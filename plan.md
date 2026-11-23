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
- [x] No stash on a district's first screen; 3 secret stashes per district (core only players' shells break)
- [x] Score per try per district (a retry starts at 0); 9 look-alike brick blocks per district hide the secrets
- [x] Map validator (owner's reports: an unkillable turret behind steel, a co-op pocket that pinned the view):
  `city_gen::sound` in the generator and over the committed districts; districts 2-5 and 7-10 redrawn; view keeps
  16 units below the trailing tank
- [x] Owner's reports (0.7.5): enemies spinning half below the view, reinforcements stuck in a turret's notch, turrets
  swinging sideways into their wall (co-op exploit), screen jump after a boss; regression tests for each
- [x] Owner's report: a reinforcement dropped into a sealed pocket in district 5; drops and placed enemies only on
  reachable cells (tested)
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
- [x] Setting KEEP POWER (stars survive losing a tank; EASY MODE since 0.7.2)
- [x] README screenshots (`docs/screenshots`, smoke hook)

### Next ideas (not started — confirm before building)
- [x] Persist hi-score (`scores.txt` in `%APPDATA%`) + world board: time bonus, stage log sent at game over to the
  owner's score server (`score_server` submodule of the Android repo), name entry, rank on the game over screen (ADR 0009)
- [x] Steam grid artwork: capsules, hero, logo, icon in `docs/steam/` (`write_steam_art`), README steps
- [x] Background music: owner's 11 tracks, embedded Ogg Vorbis, per screen / stage tier / boss (ADR 0007)
- [x] Results-screen count-up animation like the classic tally (blip per kill, A skips)
- [x] City Run world boards: district clear time + kills, one fastest-first board per district and player count,
  server checks time and kills against the district (ADR 0010)
- [x] Couch feedback: rumble on enemy hits and kills, a sad three-pulse on losing a tank; world board on its own screen
  after results / game over (A send, B skip, top 3 + 2 above + 1 below); names up to 7 letters, prefilled
- [x] HI SCORES screen on the title: world top 10 per category (classic 1P/2P, City Run district x 1P/2P), read live
  from the server's public board (`?format=text`); checked against the live server in a smoke run
- [x] Assisted runs stay off the world board: EASY MODE (was KEEP POWER) runs skip the board and achievements (0.7.2).
  Still open: FRIENDLY FIRE off in 2P counts as ranked. Runs sent since 0.4.0 carry no flag, so the boards may hold
  assisted runs
- [x] World board live check: server reachable at `scores.maxpodolski.com` (tunnel route + DNS added, key and rules checked live; runs sent and boards read from the TV and a phone)

### M8 — LAN co-op ✅ (v0.5.0, ADR 0014 + 0015)
- [x] Lockstep over UDP (packed intents, 3-tick delay), hello over TCP; `net::Driver` runs `Update` per tick
- [x] Determinism: particles' own RNG, sim loops in spawn order (`NetId`); headless two-world test with disturbance
- [x] Hash every 30 ticks, host snapshot on mismatch (checked: forced nudge healed after 7 ticks, 3.3 KB)
- [x] LAN CO-OP screen (host / join by name), linked title and pause menu, board skipped, partner loss → takeover
- [x] Two exes on one PC with bots: 4440 ticks, every hash equal
- [x] Two phones on home Wi-Fi (owner, 2026-11-19): a City Run played, one short lag
- [x] Input delay from the measured round trip, 1-5 ticks per machine (ADR 0015); loopback settles at 2
- [ ] Owner: feel of the adaptive delay on the phones
- [x] Merged to `main`, released as 0.5.0 (2026-11-19)

### M9 — Achievements ✅ (v0.6.0, ADR 0017; list in `.claude/docs/achievements.md`)
55 achievements (940 pts), on hosts with `Platform::achieve` (Android: Play Games); the exe gets none for now.
- [x] `Platform::achieve: Option<fn(&str, u32)>` (`None` in `Platform::default()`); with `None` nothing below runs
- [x] `achieve/`: the list + pure `Tracker` (streaks, losses per stage, flawless in a row, borrows, stuns, pauses,
  stars per slot); tests per rule kind and for LAN's local slot
- [x] `Feat` messages from the sim (kill, lost, mutual, stun, borrow, star, grenade, cancel, own base, secret, pause,
  score, won with the base ring check); read-only, `net::tests` green
- [x] `achievements.txt`; the cloud copy carries it after `unlocked=N` (union / max, one sync at a time)
- [x] Host hook on a worker thread, off the stage, for each change, plus everything once at start
- [x] Pixel badges for all 55 (+ locked, hidden), group plates, district-themed City Run emblems (`badge_sheet` preview)
- [x] NEW ACHIEVEMENTS screen before the world board (classic game over, City Run results / game over): animated rows
- [x] Title → ACHIEVEMENTS: tabs per group, badge rows with points and progress bars, scrollbar (smoke
  `BT_SCREEN=achievements` / `unlocks` with `BT_ACHIEVE=1`); title packs tighter with 8 items
- [x] Docs, version 0.6.0
- [x] Owner: play a few stages on the phone once Play Console has the achievements (Android plan, section M; done 2026-11-22)

### M10 — Title rework (v0.7.0, ADR 0018) ✅
- [x] Title: no tagline / START / STAGE lines; menu CLASSIC, CITY RUN, LAN CO-OP, HI SCORES, ACHIEVEMENTS, SETTINGS, EXIT
- [x] Logo in brick + steel; player tanks drive in beside it on join, back out on leave
- [x] PLAYER 1 / PLAYER 2 pixel-frame join cards beside the menu (blinking PRESS A, CONTROLLER FOUND, READY); hints per host (`Platform::keyboard`)
- [x] CLASSIC → stage tiles (6 x 5, mini-maps, padlocks), cursor only on open tiles (`stages::step` tested)
- [x] `Progress.classic` (`classic=N` in `city.txt` + cloud), frontier rule for both modes; LAN joiner keeps and saves
  their own copy (`OwnProgress`), host's in `Hello` (PROTO 3); LAN smoke: linked, hashes equal
- [ ] Owner: try the title and the tiles on the TV / phone

### M11 — Star levels 3 and 4 (v0.7.2) ✅
- [x] Star level 3 shells cut the first trees they fly into (one cell), then pass the rest (`Terrain::cut_trees`,
  `Bullet.trees`)
- [x] Star level 4: same gun, drives over water (`Terrain::blocks(r, swimmer)`, `rules::SWIM_LEVEL`)
- [x] Level 4 is armour: a hit blinks the tank 3 s (untouchable), then the stars drop; still on water then → sinks
- [x] KEEP POWER became EASY MODE: losing a tank costs one star (owner: level 4 + old KEEP POWER = OP); unranked (no
  world board, no achievements), confirm popup on turning it on
- [x] EASY MODE extras: +2 spare tanks per run / district try, 6 s respawn shield; its own unlock frontier (easy clears
  open stages only for easy mode)
- [x] Bolder looks per level: short barrel at 0, star pips on the hull from 1, floats at 4; white flash on a star
- [ ] Owner: feel on the couch (is level 4 too strong in City Run canals / moats?)

### M12 — Block-built screen titles (v0.7.4) ✅
Every menu / screen heading drawn from terrain cells like the title logo (`gfx/art_title.rs`: one glyph pixel = one
8-unit cell), not plain text.
- [x] `block_text(words, k)`: brick, steel, water, ice, trees and gold (`gold_at`, owner: achievements in gold);
  `water_at` / `ice_at` / `trees_at` split out of `art_world`; full 5x7 font (`gfx/font.rs`, A-Z 0-9 - ! ? .)
- [x] Materials: CLASSIC brick, CITY RUN / HI SCORES steel, LAN CO-OP / WORLD BOARD water, ACHIEVEMENTS / NEW
  ACHIEVEMENTS gold, SETTINGS forest, PAUSED ice, GAME OVER brick; stage banner and results: classic brick, City Run
  steel
- [x] Painted on first use and kept (`ui::Headings`), so `STAGE {n}` needs nothing special; a stage paints its
  clear / game over banners when it starts
- [x] Smoke screenshots of every screen at 16:9 (phones are wider: only more margin)
- [x] In-stage achievement star (owner): a pixel-art gold star flies in from the screen's side onto the margin beside
  the field, swells, and bursts into pixel cubes with a chime (smoke `BT_FLYOUT=<secs>`)

### M13 — Owner's stage reports (v0.7.6) ✅
- [x] Classic power-ups drop only on open ground a player can drive to (one landed in steel, one in water)
- [x] STAGE CLEAR / GAME OVER banner on a dark plate with a steel rim
- [x] Stage 6 side lanes no longer end in pockets between steel and water; stage 17 pickets no longer leave the gap
  over the base as the only way down
- [x] No quick brick-free run from a top spawn to the backline (11 stages got a brick row)
- [x] Tests for each rule over all 30 maps (`levels::tests`, `powerup::tests`)
- [ ] Owner: feel of the changed stages in 1P

### M14 — More music (v0.7.8) ✅
- [x] Owner's new tracks: slow 3-4, medium 3-5, fast 5-6, loss 2 (normalised to -16 LUFS); loss rotates
- [x] After a mini-boss or boss dies the stage music goes on with the next track, not its pool's opener (owner)

### M15 — Secret bunker (v0.7.9) ✅
- [x] Title combo left, right, left, up, up, A (D-pad + A: remotes and touch can do it) → BUNKER: CHANGELOG
  (`CHANGELOG.md`, versions 0.1.0-0.7.9), OST jukebox (24 named tracks, plays on down the list), SOUND TEST
- [x] Achievement Look Both Ways (10 pts, 950 in all; its description is the riddle), road-crossing badge
- [x] 0.7.10: A on the playing OST track pauses it, A again resumes (owner)
- [x] Owner: import the new achievement in Play Console (2026-11-22)
- [ ] Owner: rename OST tracks if the names don't fit (`music::NAMES`)

### M16 — Boss attacks + more music (v0.7.12) ✅
- [x] Owner's 9 new tracks (slow 5, upbeat slow 1-2, hardfight 3-5, bossfight 3-5), -16 LUFS Vorbis; OST names
- [x] Music shuffles after a stage's opener; mini-boss and boss fights open on a random track (owner)
- [x] District boss: charge (lit row + column) -> barrage of three shells each way; from 2/3 HP it calls in sentry
  guns (warned, fire four ways, fall with the boss) (owner: boss was a big mini-boss)
- [ ] Owner: couch feel of the boss fights (barrage gap, sentry cap, band width in `rules.rs` / `game/boss.rs`)

### M17 — Boss tricks by district + owner's reports (v0.7.13) ✅
- [x] `rules::boss_kit`: echo barrage (d3+), rage at 1/3 HP (d5+), brick ring at 1/2 HP (d7+), marked lines
  through the nearest player (d9+); districts 1-2 unchanged; phases act once (`Enemy.phase`, in the LAN snapshot)
- [x] Flashing enemies drop on the kill, not the first hit; a new drop no longer removes the one on the field
- [x] City Run respawns only where the tank can climb on without leaving the view (district 7 pocket, owner)
- [ ] Owner: couch feel of districts 7-10's bosses

## Current state

City Run added, released as v0.3.0 on GitHub: 10 districts, district map, smoke screenshots checked for the map, a district start, a
bastion and a boss arena. Not yet played on the couch.

M1–M3 done, plus Settings (FPS limit — owner saw ~1000 fps — volume, fullscreen, shake, FPS counter); all checks green (fmt, clippy, 30 tests, released as v0.1.0,
release build). Limiter measured at 59 fps (60 cap) and 140 fps (144 cap) in smoke runs. Smoke screenshots verified for the title
and stages 1, 2, 4, 5. Not yet played with real controllers — that's the first M4 step.

## Blockers

- M4 couch steps need the owner with the TV setup and real pads.
