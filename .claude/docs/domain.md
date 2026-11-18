# Domain: how the game plays

All numbers live in `src/rules.rs` unless noted.

## Field

- 13×13 tiles of 16 units (208×208). Terrain resolves to 4×4 sub-cells; maps are written in 8×8 cells.
- **Brick** blocks tanks and bullets; a bullet removes a 16-wide × 4-deep strip (8 deep at star level 3).
- **Steel** blocks everything; only star-level-3 bullets break it.
- **Water** blocks tanks, bullets fly over. **Trees** hide tanks (drawn above them), block nothing.
  **Ice**: releasing the stick keeps a tank sliding for 0.35 s.
- **Base** (bottom centre) inside a brick ring. Any bullet on it — including a player's — loses the game.

## Tanks

- Movement is 4-way. Turning onto the other axis snaps the old axis to the 8-unit grid (only if that spot is
  free), so 16-wide tanks fit 16-wide gaps — the classic feel. Tanks stop flush against obstacles (bisection).
- Tanks block each other, except pairs already overlapping (e.g. after a spawn) may drive apart.
- Spawning takes 1 s (sparkle); spawning tanks block others but can't be hit or move.

### Players
- 2 spare tanks (3 total); +1 every 20 000 points; tank power-up +1.
- Star levels: 0 slow bullet · 1 fast bullet · 2 two bullets · 3 breaks steel / digs deeper. Dying resets to 0.
- Respawn shield 3 s; helmet 10 s. Hit by the other player → paralyzed 3 s (no move/fire), no damage — only with FRIENDLY FIRE on.
- Fire on press, auto-repeat every 0.25 s while held, capped by bullets in flight.

### Enemies (20 per stage, roster per stage file, shuffled deterministically per stage number)
| Type (results label) | Speed | Bullet | HP | Points |
|---|---|---|---|---|
| Basic (LIGHT) | 30 | slow | 1 | 100 |
| Fast (SCOUT) | 66 | slow | 1 | 200 |
| Power (GUNNER) | 40 | fast | 1 | 300 |
| Armor (HEAVY) | 32 | medium | 4 | 400 |

- Spawn points top-left/centre/right in turn; skip an occupied one. Interval 3.2 s − 0.06·stage − 0.4 in 2P (≥ 1 s).
- At most 4 on the field (1P) / 6 (2P). Enemies #4, #11, #18 flash red and drop a power-up when first hit.
- AI: on a timer or when blocked, 45 % head for a target (base 65 %, nearest player 35 %) along the longer
  axis, else weighted wander (down 40 %, left/right 20 % each, up 20 %); never re-picks a blocked heading.
  Fires every 0.4–1.6 s, and right away half the time it gets blocked. Enemy bullets pass through enemies.

## Bullets
- Sub-stepped ≤ 2 units. Order per step: tank hit → base → terrain/edge.
- Player and enemy bullets cancel each other (swept rectangles). Two players' bullets don't.

## Power-ups (one on the field; a new drop replaces the old; +500 points)
Helmet (shield 10 s) · Clock (enemies freeze 10 s, tinted icy) · Shovel (steel base ring 20 s, blinks back to
brick in the last 3 s) · Star (+1 level) · Grenade (destroys all enemies on the field, no points) · Tank (+1 life).

## Flow
Title → join (A / Space / Enter) → START (or pick STAGE n first) → stage banner 2.2 s → play →
all 20 destroyed → 3 s → results tally → next stage. Base lost or every player out → 3 s → game over → title.
30 stages, rosters ramping from mostly LIGHT (1) to mostly HEAVY (30). After stage 30 the maps loop; each lap converts
4 weaker enemies into stronger ones.

**Map rule (tested):** from every enemy and player spawn, a 16×16 tank must be able to reach the base treating
bricks as open and steel/water as walls (`levels::tests::every_spawn_can_reach_the_base`). Watch for: a water gap
that exits straight into steel, and 1-cell (8-unit) open bands between obstacle rows — a tank needs 2 cells.

## Controls
| | Controller | Keyboard P1 | Keyboard P2 |
|---|---|---|---|
| Move | D-pad / left stick | WASD | Arrows |
| Fire / confirm | A (also X, RB, RT fire) | Space / F | Enter / Right Ctrl / Num 0 |
| Back / leave slot | B | Backspace | Backspace |
| Pause | Start | Esc | Esc |
| Fullscreen | Settings | F11 / Alt+Enter | |

## Settings
The title shows the build version (`Cargo.toml` `version`) bottom-right.

Title → SETTINGS (any controller or keyboard; left/right or A changes, B back). Saved on every change to
`%APPDATA%\BattleTanks\settings.txt` as `key=value` lines; bad or missing values fall back to defaults.
| Setting | Values | Default | Notes |
|---|---|---|---|
| FPS LIMIT | 30/60/90/120/144/165/240/OFF | 60 | own limiter (sleep, spin last 1.5 ms) — works even when VSync is ignored |
| VSYNC | ON/OFF | ON | `AutoVsync` / `AutoNoVsync` |
| FULLSCREEN | ON/OFF | ON | borderless; F11 / Alt+Enter flips and saves it; `--windowed` forces OFF (saved if you then change a setting) |
| VOLUME | 0–100 % in 10s | 80 % | 0 skips playback entirely |
| SCREEN SHAKE | ON/OFF | ON | comfort on a big TV |
| SHOW FPS | ON/OFF | OFF | smoothed FPS, top-right, every screen |
| FRIENDLY FIRE | ON/OFF | ON | ON = classic (players paralyze each other); OFF = player bullets pass through the other player |
| VIBRATION | ON/OFF | ON | rumble on the slot's own pad: own tank destroyed (strong), paralyzed (medium), base lost (everyone, long), kill / power-up (short tick) |

Controller disconnect → game pauses with a note; any unassigned controller pressing A takes over the slot.

## Known gaps / deliberate omissions
- Hi-score lives for the session only (not saved).
- No music loop, no Steam grid artwork (the exe and window do have an icon).
- Results tally is static (no count-up animation).
- Grenade-killed spawning enemies leave their sparkle to finish (cosmetic).
- Keyboard layouts share Esc and Backspace; both act for whoever is joined on the keyboard.
- Variable timestep (dt clamped 0.05 s) rather than fixed; fine at 60 fps vsync, revisit if behaviour drifts
  at very low frame rates.
- No online play, no AI teammate, no level editor (maps are plain text).

## Operational lessons
| Symptom | Cause | Guard |
|---|---|---|
| `bevy_a11y = "^0.20.1"` not found | stale local crates.io index cache | delete `Cargo.lock`, `cargo generate-lockfile` |
| Startup panic "Resource does not exist" | `OnEnter(Title)` runs before `PreStartup` commands apply | build `Gfx` in the plugin fn, not a startup system |
| One Esc press joined both keyboard slots | Esc is "start" on both layouts | join only on confirm, never on start |
| Armor enemy looked like player 2 | green tint shared with 2P | enemy palette never uses gold/green |
| ~1000 fps on the owner's PC despite `AutoVsync` | VSync not honoured (driver override / streaming setup) | own FPS limiter in Settings, default 60 |
| New stages walled spawns off from the base | bridge exits into steel; 1-cell open bands between blob rows | reachability test over all stages |
| Smoke runs blasted sound in the room | audio always on | audio plugin skips when `BT_SHOT` / `BT_MUTE` is set |
