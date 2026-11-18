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
- [ ] Couch-test difficulty and length; boss HP bar if bosses feel like sponges

### M4 — Ship to the TV ⏳
- [x] Release profile (thin LTO, stripped, no console window)
- [ ] Build release exe on the gaming PC, add to Steam (see README), test through Sunshine/Moonlight with 2 pads
- [ ] Couch tuning pass: enemy speeds, spawn interval, max enemies in 2P, AI aggression (`rules.rs`, `enemy.rs`)
- [ ] Check Steam Input doesn't double controllers (disable per-game Steam Input if it does)

### M6 — Android TV + Google Play ⏳ (planned, ADR 0006)
Same game, second target. The Windows exe keeps working at every step (checks green after each phase).

**A. One crate, two targets** (no behaviour change on Windows)
- [ ] `src/lib.rs` with `pub fn run()`; `main.rs` calls it; `[lib] crate-type = ["cdylib", "rlib"]`
- [ ] `build.rs` icon step only when `CARGO_CFG_TARGET_OS == "windows"`
- [ ] `settings::data_file()` on Android uses `AndroidApp::internal_data_path()` (now `%APPDATA%`, which is `None` there)
- [ ] Android-only deps: bevy `android-game-activity` + `gles` (fallback for TVs with weak Vulkan), `jni`

**B. Hello TV**
- [ ] `android/` Gradle project: `MainActivity extends GameActivity`, `minSdk` 26, `targetSdk` = what Play currently requires
  (API 36 from Aug 2026, check in Play Console), ABIs `arm64-v8a` + `armeabi-v7a` (many TVs, e.g. Sony Bravia, run 32-bit ARM)
- [ ] TV manifest: `LEANBACK_LAUNCHER`, `uses-feature android.software.leanback` / `touchscreen required=false` /
  `android.hardware.gamepad required=false`, landscape, 320×180 banner painted by `gfx/icon.rs` like the exe icon
- [ ] Build: `cargo ndk -t arm64-v8a -t armeabi-v7a -o android/app/src/main/jniLibs build --release --lib`, then `gradlew assembleDebug`
- [ ] 16 KB page alignment (NDK r28+ default; check with `zipalign -c -P 16`)
- [ ] Smoke on the Android TV emulator: debug-only intent extras → `BT_SHOT`/`BT_STAGE`/`BT_SHOT_AT`, pull the PNG with `adb`
- [ ] `AppExit` (title EXIT, pause → EXIT GAME) finishes the activity

**C. Controllers: standard pads + TV remote** (Java hub → JNI → Bevy raw gamepad events)
- [ ] `PadHub.java`: consume `SOURCE_GAMEPAD|JOYSTICK|DPAD` key/motion events; stable id per `InputDevice`; connect/disconnect via `InputManager.InputDeviceListener`
- [ ] Mapping: A/B/X/Y, shoulders, triggers (`AXIS_LTRIGGER/RTRIGGER`, fall back to `AXIS_BRAKE/GAS`), sticks, hat → D-pad, Start/Select;
  TV remote = pad with D-pad, centre = South, Back = East (Back must never quit the activity during play)
- [ ] `src/android.rs`: JNI `nativePadEvent` → queue → PreUpdate system writes `GamepadConnectionEvent` / `RawGamepadEvent`
  (before Bevy's gamepad processing); `GamepadRumbleRequest` → `PadHub.rumble()` (`VibratorManager`, API 31+; skip below)
- [ ] Tests: pure keycode/axis → Bevy button/axis table (Rust), event-queue → raw events
- [ ] Check on hardware: Xbox Series (BT + USB), Xbox One, DualSense, generic HID, TV remote; two pads at once; unplug → pause → takeover still works

**D. Steam Controller (2026 "Triton")**: bring over the owner's own driver from moonlight-android PR #623
- [ ] Copy `SteamTritonController` (report parsing), `SteamControllerBle`, `SteamControllerUsb` and their test, re-headed under this repo's
  licence (owner's code; nothing else from Moonlight), protocol checked against SDL's Triton driver (zlib)
- [ ] Driver feeds `PadHub` like any pad: lizard mode off, swallow its duplicate mouse/keyboard events, left trackpad → left stick
  (whichever pushes further wins), rumble; `BLUETOOTH_CONNECT` runtime permission (API 31+), `android.hardware.usb.host required=false`
- [ ] Start on activity resume for bonded "Steam Ctrl" devices + puck/USB attach; release on pause so the controller works normally outside the game
- [ ] Hardware check: BLE on the Bravia (Android 14, 32-bit); puck and cable

**E. Android polish**
- [ ] Lifecycle: going to the background auto-pauses the stage, stops audio, and restores both on resume
- [ ] Settings: hide FULLSCREEN and VSYNC on Android (the OS owns them); keep FPS LIMIT, VOLUME, SHAKE, VIBRATION, FRIENDLY FIRE
- [ ] Performance on a weak TV box (bloom/HDR cost on GLES); add a LOW EFFECTS setting only if it misses 60 fps

**F. Google Play**
- [ ] Developer account (one-time fee, identity check). New personal accounts must run a **closed test with 12+ testers for 14 days**
  before production, so start this early
- [ ] Release AAB (`gradlew bundleRelease`), Play App Signing, upload key outside the repo (gitignored `android/keystore.properties`);
  `versionName` from `Cargo.toml`, `versionCode` derived from it
- [ ] Listing: 512 px icon, 1024×500 feature graphic, TV banner, 1920×1080 TV screenshots (smoke hook), description stating "controller
  recommended, TV remote supported"
- [ ] Content rating (IARC), data safety (collects nothing; saves stay on the device), privacy-policy page (README section or GitHub Pages)
- [ ] Opt in to the Android TV form factor in Play Console → TV quality review (remote navigation, banner, no touch prompts, Back behaviour)

**G. Release flow**
- [ ] One tag builds both: `BattleTanks.exe` + `BattleTanks.apk` (sideload) on the GitHub release; the AAB goes to Play
- [ ] Docs: CLAUDE.md commands (Android build/smoke), architecture tree (`android/`, `src/android.rs`), domain controls (TV remote, Steam Controller), lessons

### Next ideas (not started — confirm before building)
- [ ] Persist hi-score (small file in `%APPDATA%`)
- [ ] Steam grid artwork (capsule/hero images)
- [ ] Background music loop
- [ ] Results-screen count-up animation like the classic tally

## Current state

City Run added, released as v0.3.0 on GitHub: 10 districts, district map, smoke screenshots checked for the map, a district start, a
bastion and a boss arena. Not yet played on the couch.

M1–M3 done, plus Settings (FPS limit — owner saw ~1000 fps — volume, fullscreen, shake, FPS counter); all checks green (fmt, clippy, 30 tests, released as v0.1.0,
release build). Limiter measured at 59 fps (60 cap) and 140 fps (144 cap) in smoke runs. Smoke screenshots verified for the title
and stages 1, 2, 4, 5. Not yet played with real controllers — that's the first M4 step.

## Blockers

- M6 hardware steps (C/D checks, performance) need the owner with a TV box and real pads; agents can only use the Android TV emulator.
- M6 F needs the owner's Play developer account and 12 closed-test testers.
