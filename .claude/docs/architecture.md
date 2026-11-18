# Architecture

## Source tree

```
Cargo.toml            # bevy 0.20 (2d, ui, bevy_audio, wav, png); dev deps opt-level 3; release thin LTO + strip
levels/NN.txt         # stage maps, baked into the exe with include_str! (format in levels.rs)
src/
  main.rs             # loads Settings first so the window opens in the saved mode; plugin list
  geom.rs             # units (SUB 4, CELL 8, TILE 16, FIELD 208), Dir, rect overlap, grid snap  [pure]
  rng.rs              # xorshift64* Resource: below/range/chance/pick/shuffle                     [pure]
  settings.rs         # Settings (fps, vsync, fullscreen, volume, shake, show_fps, friendly_fire, vibration) <-> %APPDATA%\BattleTanks\settings.txt;
                      # apply to window, F11 toggle, FPS overlay, frame limiter in Last
  rules.rs            # every gameplay number: speeds, timers, enemy stats, gun levels, scoring    [pure]
  terrain.rs          # 52x52 sub-cell grid, tank blocking, bullet impact strip, base ring, dirty list [pure]
  levels.rs           # parse stage files, force spawns/base clear, rosters, harder laps         [pure]
  input.rs            # Device (pads, two keyboard layouts) -> Intent per frame (DeviceIntents); Rumble -> pad
  audio.rs            # synthesises WAVs at startup; plays Sfx messages (deduped per frame); BT_SHOT/BT_MUTE mute
  dev.rs              # BT_SHOT smoke hook: auto-join, start stage, screenshot, log average fps, quit
  game/
    mod.rs            # Screen state, Session/Slot, StageRun, components, Fx/Sfx messages, Sim schedule
    stage.rs          # OnEnter build, enemy spawner, timers (shield/spawn/freeze/shovel), win/lose, pause, pad loss
    tank.rs           # player_control (intent -> want/trigger), drive (collisions, snap, ice), fire
    enemy.rs          # think: choose_dir (pure) + random fire, freeze
    combat.rs         # fly_bullets (sub-steps: tanks, base, terrain; bullet-vs-bullet) -> Hit; resolve_hits
    powerup.rs        # drop on bonus hit, pickup effects
  gfx/
    mod.rs            # Gfx handles (built at plugin time), colours, z layers, camera (FixedVertical 224) + bloom
    canvas.rs         # SDF painter -> Image; sd_box/rbox/circle/seg/poly, star, tileable noise   [pure]
    art_tanks.rs      # 4 player + 4 enemy tank models x 2 tread frames, grey (tinted by Sprite.color)
    art_world.rs      # brick, steel, water x3, trees, ice, ground, base (alive/destroyed)
    art_icons.rs      # bullet, particle dot, spawn sparkle, shield ring, flag, 6 power-up icons
    world.rs          # terrain sub-cell sprites from the dirty list, water anim, attach/update tank/bullet/base/power-up sprites
    fx.rs             # Fx messages -> particles, score pop-ups (Text2d), camera shake
  ui/
    mod.rs            # text() in vh units, colours, menu helpers
    title.rs          # join/leave slots, START / STAGE <n> / SETTINGS / EXIT
    settings.rs       # rows from settings::ITEMS + BACK; saves on every change
    hud.rs            # side panels: lives, score, power, enemy reserve icons, stage flag
    screens.rs        # stage banner, pause menu (+ pad-lost note), results tally, game over
```

## Frame pipeline

```
PreUpdate   input::gather ──▶ DeviceIntents [(Device, Intent)]
                │
Update          ├─ title / pause / results / game-over menus read DeviceIntents (via Session slots)
                │
                ├─ stage: watch_controllers ─▶ toggle_pause ─▶ advance_phase (Intro→Play, Won/Lost→next Screen)
                │
                └─ Sim (only: Screen::Stage, !Paused, phase != Intro), chained:
                   spawn_enemies ─▶ tick_timers ─▶ player_control ─▶ enemy::think ─▶ drive ─▶ fire
                   ─▶ fly_bullets ──Hit──▶ resolve_hits ─▶ powerup::pickup ─▶ check_end
                         │  │                    │
                         │  └── Terrain.dirty ───┼──────────────▶ gfx::world::sync_terrain (spawn/despawn chips)
                         └──── Fx / Sfx / Rumble ┘──────────────▶ gfx::fx (particles, shake)   audio::play, input::rumble (PostUpdate)
                after(Sim): gfx::world attach_* (Added<T>) + tank_looks/shield_rings/... ; ui::hud::refresh
```

## State machine

```
Settings ◀──SETTINGS / B──▶ Title ──START──▶ Stage ──all 20 killed──▶ Results ──A / 10 s──▶ Stage (n+1)
                             ▲                │
                             │                └──base destroyed or everyone out──▶ GameOver ──A──▶ Title
                             └────────────── pause menu "QUIT TO TITLE" ◀──────────┘
```

`StageRun.phase` inside `Screen::Stage`: `Intro` (banner, frozen) → `Play` → `Won` | `Lost` (3 s, world keeps
running) → next screen. `Paused` is a separate resource; the pause overlay is part of the stage UI.
