# Battle Tanks — agent guide

Two-player couch tank game in the spirit of the 8-bit "defend the base" tank classics, built with **Rust + Bevy 0.20**.
Primary target: a Windows PC streamed to a TV (Sunshine / Vibepollo → Moonlight), launched from Steam as a
non-Steam game, played with **controllers**. Two players on one keyboard also work.

@plan.md
@.claude/docs/architecture.md
@.claude/docs/domain.md

## Commands (run from repo root)

| Task | Command |
|------|---------|
| Build (debug) | `cargo build` |
| Test | `cargo test` |
| Lint | `cargo clippy --all-targets` (must be warning-free) |
| Format | `cargo fmt` (`rustfmt.toml`: width 140) |
| Release exe | `cargo build --release` → `target/release/battle_tanks.exe` (single file, no assets) |
| Redraw City Run districts | `cargo test write_city_districts -- --ignored` → `levels/city/*.txt` (commit; overwrites hand edits) |
| Achievement badge sheet (look at it) | `BT_BADGE_SHEET=<scratch>/badges.png cargo test badge_sheet -- --ignored` |
| Regenerate app icon | `cargo test write_icon_file -- --ignored` → `assets/icon.ico` + `icon.png` (commit both) |
| Smoke run + screenshot | `BT_SHOT=<scratch>/shot.png BT_STAGE=2 BT_SHOT_AT=8 ./target/debug/battle_tanks.exe --windowed` |
| LAN smoke (two copies) | run twice at once: `BT_LAN=host` / `BT_LAN=join`, both with `BT_LAN_BOT=1 BT_LAN_SECS=60 BT_MUTE=1 ... --windowed`; compare their `lan: tick <t> hash <h>` log lines (`BT_DESYNC=<tick>` on the joiner tests the resync) |

- First build compiles Bevy (~3 min). If cargo says a `bevy_*` version "doesn't exist", the local index is stale:
  delete `Cargo.lock` and run `cargo generate-lockfile`.
- **Agents build and test; they don't play.** The only allowed run is the smoke hook above (auto-joins two
  keyboard players, screenshots, exits by itself; sound is muted while `BT_SHOT` is set, `BT_MUTE=1` mutes
  any run). Use it after visual or startup changes and *look at the PNG*.
  `BT_STAGE=0` screenshots the title (`BT_SCREEN=joined` with player 1 joined, `play` the mode cards, `stages` the classic stage tiles (9 open), `settings` the settings screen, `achievements` the list, `unlocks` the NEW ACHIEVEMENTS screen, `results` / `gameover` / `cityresults` / `board` a made-up run, `city` the district map, `lan` LAN CO-OP, `hiscores`, `bunker` the secret bunker (`BT_TAB=1` OST, `2` sound test); `BT_ACHIEVE=1` turns achievements on with logged host calls). `BT_VERSUS=<map>` starts a VERSUS round on that map, `BT_SCREEN=versus` the VERSUS results, `versussetup` VERSUS SETUP, `draft` the army draft. In a stage `BT_PAUSE=1` opens the pause menu and `BT_FLYOUT=<secs>` fakes two unlocks that long before the shot (achievement stars). It logs `smoke: N fps average` on exit. Write screenshots to the scratchpad, never the repo.

## Tech stack

- Rust 2024 edition, Bevy 0.20.1 with `default-features = false` + `2d`, `ui`, `bevy_audio`, `wav`, `png`.
- Gamepads via Bevy's gilrs backend (XInput/WGI on Windows — what Sunshine's virtual pads and Steam Input expose).
- No other dependencies. RNG is our own xorshift (`rng.rs`). Art and sound are generated at startup (ADR 0002).
  The world board shells out to the OS `curl` (ADR 0009); its game key lives in the gitignored `scores.key` (ask the
  owner; never commit it). Without the file the build is simply offline.
- Bevy 0.20 is newer than most training data: **check the API in `~/.cargo/registry/src/*/bevy_*-0.20.*`
  before guessing** (e.g. `Message`/`MessageWriter` replaced buffered events, `DespawnOnExit`, `FontSize::Vh`,
  `CursorOptions` is separate from `Window`).

## Principles

- **Gameplay stays classic, visuals get better.** Grid-snapped 4-way tanks, destructible bricks, steel, water,
  trees, ice, a base to defend, 20 enemies per stage, power-ups. Don't "modernise" the rules without asking;
  tune numbers in `rules.rs`.
- **Original content only.** Our own stage maps, art, sounds, name. Never import sprites, maps or audio from
  existing games or rips (ADR 0003), even when a repo with them is offered as reference.
- **Controller first, TV first.** Every screen is usable with a pad (A confirm, B back, Start pause, D-pad/stick).
  Text is sized in `vh` units. The cursor is hidden. There is always a way to quit from a pad (title EXIT,
  pause → EXIT GAME), because the game runs fullscreen on a TV.
- **Logic entities have no sprites.** `game/` spawns plain components; `gfx/` attaches visuals when they appear
  (`Added<T>`) and reads `Fx`/`Sfx` messages. Gameplay never touches `Sprite`.
- **Pure rules in plain functions** (`geom`, `terrain`, `levels`, `rules`, `enemy::choose_dir`,
  `tank::advance`, `combat::can_hit`, `canvas` SDFs) with unit tests in the same file.
- **Public API is a contract.** A wrapper crate outside this repo builds on `run_with(Platform, extra plugins)`,
  `Platform`, `TvRemote`, `canvas`, `paint_icon`, `paint_banner`, `ACHIEVEMENTS` and `badge_pixels`. Change them only on purpose; keep `Platform::default()` = the exe.
- Lazy and small: stdlib/Bevy before new crates, shortest diff, `// ponytail:` comments on deliberate
  shortcuts naming the ceiling and the upgrade path.

## Conventions

- One file ≈ one responsibility, **≤ ~300 lines**; split when it grows (see `gfx/art_*.rs`).
- Plugins are functions: `pub fn plugin(app: &mut App)`. Each module registers its own systems.
- Gameplay systems live in the `Sim` set (one `.chain()` in `game/mod.rs`); it only runs while a stage is
  live, unpaused and past the intro banner. Visual systems run `.after(Sim)`.
- Everything spawned for a stage gets `DespawnOnExit(Screen::Stage)`; screens likewise for their state.
- Field coordinates = world coordinates: origin bottom-left of the 208×208 field, y up. Terrain grid is
  52×52 sub-cells of 4 units; map files are 26×26 cells of 8 units.
- Variable timestep, `dt` clamped to 0.05 s; bullets sub-step (≤ 2 units) so they can't tunnel. In a LAN session
  `Update` runs once per lockstep tick with a fixed 1/60 s (ADR 0014).
- **Determinism (LAN, ADR 0014):** gameplay randomness only from the gameplay `Rng`, never from visuals; a sim loop
  whose result depends on order (dice per entity, first match, sequential moves) iterates
  `.sort_by_key::<Entity, _>(|e| order(&ids, *e))` (spawn order, `NetId`); new sim state goes into `net::snap`.
  `net::tests` fails when two worlds drift.
- Crate-level `allow(clippy::type_complexity, clippy::too_many_arguments)` — normal for Bevy queries.
- Comments say *why*. No commented-out code.

## Docs ship with the change

| Change | Update |
|--------|--------|
| New module / file move / new system set | `.claude/docs/architecture.md` tree + frame diagram |
| Rule, number, control or flow change | `.claude/docs/domain.md` |
| Something deliberately left out | "Known gaps" in `domain.md` |
| A bug that bit us | one line in "Operational lessons" in `domain.md` |
| Engine/library/architecture decision that would hurt to reverse | new `.claude/docs/adr/NNNN-*.md` (never edit an accepted one; supersede) |
| Milestone progress | `plan.md` checklist |

## Tests and definition of done

- Every change ships with tests for the pure logic it touches; every bug fix gets a regression test.
- Done = `cargo fmt` clean, `cargo clippy --all-targets` warning-free, `cargo test` green, `cargo build --release`
  succeeds, and a smoke screenshot looked at when visuals/startup changed. Then copy
  `target/release/battle_tanks.exe` to `<folder>\BattleTanks.exe`, where `<folder>` is the single line in the
  gitignored `target_game_folder.env` (the owner's local game folder; skip if the file is absent). Then commit straight to `main` and
  push. Releases: bump `version` in `Cargo.toml` (shown on the title; only the patch number, 0.7.2 -> 0.7.3, never
  the minor unless the owner says so), tag `vX.Y.Z`, `gh release create` with
  the release exe attached as `BattleTanks.exe` (no branches/PRs unless asked, no AI co-author trailers).
