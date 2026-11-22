# Domain: how the game plays

All numbers live in `src/rules.rs` unless noted.

## Field

- 13×13 tiles of 16 units (208×208). Terrain resolves to 4×4 sub-cells; maps are written in 8×8 cells.
- **Brick** blocks tanks and bullets; a bullet removes a 16-wide × 4-deep strip (8 deep at star level 3).
- **Steel** blocks everything; only star-level-3 bullets break it.
- **Water** blocks tanks (except a star-level-4 player), bullets fly over. **Trees** hide tanks (drawn above them),
  block nothing. A star-level-3+ shell cuts one map cell of trees (16 wide, 8 deep) where it first flies into them,
  then flies on through any further trees untouched.
  **Ice**: releasing the stick keeps a tank sliding for 0.35 s.
- **Base** (bottom centre) inside a brick ring. Any bullet on it — including a player's — loses the game.

## Tanks

- Movement is 4-way. Turning onto the other axis snaps the old axis to the 8-unit grid (only if that spot is
  free), so 16-wide tanks fit 16-wide gaps — the classic feel. Tanks stop flush against obstacles (bisection).
- Tanks block each other, except pairs already overlapping (e.g. after a spawn) may drive apart.
- Spawning takes 1 s (sparkle); spawning tanks block others but can't be hit or move. Players' tanks at a stage's
  start skip it: they are there under the banner, with the 3 s respawn shield, and move the moment it lifts.

### Players
- 2 spare tanks (3 total); +1 every 20 000 points; tank power-up +1.
- **Borrowing** (both modes): a player who is out presses A (fire/confirm) to take one of the partner's spare
  tanks and respawn at once. The HUD shows `A: BORROW A TANK` while that is possible.
- Star levels: 0 slow bullet · 1 fast bullet · 2 two bullets · 3 breaks steel / digs deeper / cuts the first trees ·
  4 same gun, drives over water, and is armour. Losing a tank resets to 0 (in EASY MODE: one star less).
- **Armour (level 4)**: an enemy hit doesn't destroy the tank. It blinks for 3 s, untouchable (also by the partner's
  shells), then loses its stars as if the tank were lost (to 0, or to 3 in EASY MODE). Still on water when the
  blinking ends (level 3 can't swim): it sinks and counts as a lost tank.
- The tank's look grows with the level: level 0 has a short barrel, level 1 a long one, 2 twin barrels, 3 a heavy hull
  with side plates, 4 adds floats on the treads; from level 1 bright dots on the hull's back count the stars. Picking up
  a star flashes the tank white for 0.25 s.
- Respawn shield 3 s; helmet 10 s. Hit by the other player → paralyzed 3 s (no move/fire), no damage — only with FRIENDLY FIRE on.
- Fire on press, auto-repeat every 0.25 s while held, capped by bullets in flight.

### Enemies (20 per stage, roster per stage file, shuffled deterministically per stage number; the first 3 spawns are the roster's weakest)
| Type (results label) | Speed | Bullet | HP | Points |
|---|---|---|---|---|
| Basic (LIGHT) | 30 | slow | 1 | 100 |
| Fast (SCOUT) | 54 | slow | 1 | 200 |
| Power (GUNNER) | 40 | fast | 1 | 300 |
| Armor (HEAVY) | 32 | medium | 4 | 400 |

- Spawn points top-left/centre/right in turn; skip an occupied one. Interval 3.2 s − 0.06·stage − 0.4 in 2P (≥ 1 s).
- At most 4 on the field (1P) / 6 (2P). Enemies #4, #11, #18 flash red and drop a power-up when first hit.
- AI: on a timer (0.8–3 s; scouts 1.4–4 s so they don't jitter) or when blocked, 45 % head for a target (base 65 %, nearest player 35 %) along the longer
  axis, else weighted wander (down 40 %, left/right 20 % each, up 20 %); never re-picks a blocked heading.
  Fires every 0.4–1.6 s, and right away half the time it gets blocked. Enemy bullets pass through enemies.

## Bullets
- Sub-stepped ≤ 2 units. Order per step: tank hit → base → terrain/edge.
- Player and enemy bullets cancel each other (swept rectangles). Two players' bullets don't.

## Power-ups (one on the field; a new drop replaces the old; +500 points)
Classic drops land on open ground (no brick, steel or water) that a tank can reach from player 1's spawn without
crossing steel or water (tested). Helmet (shield 10 s) · Clock (enemies freeze 10 s, tinted icy) · Shovel (steel base ring 20 s, blinks back to
brick in the last 3 s) · Star (+1 level) · Grenade (destroys all enemies on the field, no points) · Tank (+1 life).

## Screen headings (`gfx/art_title.rs` `block_text`, `ui::Headings`)
Every screen heading is built from terrain blocks like the logo, one 5x7 glyph pixel per 8-unit cell: CLASSIC brick,
CITY RUN and HI SCORES steel, LAN CO-OP and WORLD BOARD water, ACHIEVEMENTS and NEW ACHIEVEMENTS gold, SETTINGS
forest, PAUSED ice, GAME OVER brick. The stage banner (STAGE n / STAGE CLEAR) and the results heading are brick in
classic and steel in City Run (DISTRICT n with the district's name under it); in a stage they (and GAME OVER) sit on a dark
plate with a steel rim, so they read over a busy field. Water and trees are lifted brighter
than on the field so they read on black; the banner darkens the field more (85 % at the start, 55 % at the end) so a
brick heading reads over bricks.

## Title (`ui/title.rs`, ADR 0018)
- Menu: CLASSIC, CITY RUN, LAN CO-OP (hosts with `Platform::lan`), HI SCORES, ACHIEVEMENTS (hosts with
  `Platform::achieve`), SETTINGS, EXIT. CLASSIC and CITY RUN are dimmed until someone has joined.
- The logo is built from terrain: "BATTLE" in brick, "TANKS" in steel. When a player joins, their tank drives in
  from their side of the screen (1.8 s, slowing as it parks) and parks beside the logo; it backs out when they leave.
- A **card per player** (pixel-art frame in the player's colour): PLAYER 1 left of the menu, PLAYER 2 right. Joining: A (Space / Enter) from any device that hasn't
  joined takes the first free card; B leaves it.
  - Empty card: a dim tank and a blinking "PRESS A TO JOIN" ("or Space / Enter" on hosts with a keyboard). When a pad
    is on but in no slot, the card says "CONTROLLER FOUND".
  - Joined card: the device ("CONTROLLER", "KEYS WASD", or the machine's name over LAN) and "READY   B leave".
- **Secret bunker** (offline only): on the title press left, right, left, up, up, A ("look left, right, left again,
  cross twice, honk": D-pad + A only, so a TV remote and the touch controls can do it; no B, which leaves a slot or
  quits a TV app from here). Any devices' presses count, in order; the A that finishes it neither joins nor picks a
  menu item. It opens the BUNKER (steel heading), three tabs, left/right to switch, B back:
  CHANGELOG (`CHANGELOG.md`, up/down scrolls), OST (all 24 tracks with a name and where they play; A plays one, then
  the next down the list when it ends; leaving gives the screen its own music back; follows the MUSIC volume, at 0 it
  says so) and SOUND TEST (every sound effect, A plays). Getting in unlocks Look Both Ways (FUN), whose description
  is the combo's riddle, so the achievement list is the hint.
- Under the menu: the local hi score and the controls. The pad line is always shown; the keys line only with
  `Platform::keyboard`.

## Classic stage select (`ui/stages.rs`, `game/progress.rs`, ADR 0018)
CLASSIC → a 6 × 5 grid of stage tiles, each showing its map in miniature with the stage number. Cleared tiles have a
gold frame, the next unbeaten one a white frame; the rest are dark with a padlock. Stage 1 is open from the start.
Clearing the highest open stage opens the next; replays and clears past it open nothing. The cursor moves only over
open tiles and starts on the highest: left/right step through the stages in order, up/down jump a row, A starts,
B goes back. Saved as `classic=N` in `city.txt` and the cloud copy (see City Run). EASY MODE keeps its own frontier
(see Settings).

## Flow
Title → join (A / Space / Enter) → CLASSIC → pick a stage tile → stage banner 1.2 s (world frozen) → play →
all 20 destroyed → 3 s → results tally → next stage. Base lost or every player out → 3 s → game over → title.
30 stages, rosters ramping from mostly LIGHT (1) to mostly HEAVY (30). After stage 30 the maps loop; each lap converts
4 weaker enemies into stronger ones.

## Scoring and the world board (`rules.rs`, `scores.rs`, `ui/board.rs`, ADR 0009)
- Points: kills (100–400 by type), power-ups 500, and a **time bonus** per cleared classic stage, to each player:
  fastest possible clear = 19 × spawn interval + 1 s (the 20th enemy can't arrive sooner); par = fastest + 120 s;
  20 points per second under par, rounded down to tens (max 2400). The clock runs only in play (no banner, no pause)
  until the last kill. City Run has no time bonus; its clear time is what its world boards rank.
- **Results tally** counts up like the classics: row by row, one blip per kill (0.07 s each, 0.35 s between rows), then
  the stage time with the bonus, then the score. A skips the count; then A (after 1 s) or 10 s moves on.
- **Hi score** is kept in `%APPDATA%\BattleTanks\scores.txt` (`hi=`, plus `name1=` / `name2=`: the name last
  entered per slot, prefilled next time), saved on results, game over and the title.
- **World board** (builds with a game key, host with `post_score`): its own screen after the game over screen (A).
  Each player sets a name of up to 7 letters with their own controller, prefilled with their last one (up/down letter,
  A-Z then blank; left/right move; A sends, in 2P once both pressed A; B skips to the title, nothing sent).
  The run's stage log goes to `scores.maxpodolski.com` (game `battletanks`), which recomputes and checks it and keeps
  the top 100 per board (1P, 2P co-op team total). The screen then shows the world rank, the top 3, the two rows above
  ours and the one below, or "offline - score not sent"; A moves on. Runs quit from the pause menu are not sent.
- **City Run boards** (ADR 0010): the district results screen (it shows `TIME m:ss.t`) is followed by the same board
  screen (B skips to the district map), which sends the clear: play time (to the boss kill) and kills per player, ordinary tanks by type plus turrets,
  mini-bosses and the boss. One board per district and player count, fastest first. The server refuses a time below
  the straight drive from the start to the boss at full speed, a clear that does not kill the boss, every turret
  and every mini-boss, or more kills than the district plus its reinforcements hold in that time. The district numbers
  live on both sides (`scores.rs` `CITY_FACTS`, server `CityRules.Facts`); the test
  `city_facts_match_the_maps_and_the_server` fails on a map change until both are updated.

- **HI SCORES** (title menu, between CITY RUN and SETTINGS; any pad, remote or keyboard): the world
  board's top 10 for one category at a time. Selectors: MODE (CLASSIC / CITY RUN), PLAYERS (1 / 2), and in City Run
  DISTRICT (1-10 with its name). Up/down picks a selector, left/right (or A) changes it, B goes back; the last board
  looked at is kept until the game closes. Columns: rank, names, score + stages cleared (City Run: clear time + kills).
  Ranks 1-3 are gold, silver and bronze; rows with a name this machine last entered are green. Each board is read once
  per visit (`GET ...scores?format=text&take=10&board=<id>`, no key needed): "loading...", "no runs yet - be the
  first!", or "can't reach the world board - A to retry". Classic boards also show this machine's local best. A host
  without `Platform::fetch_board` shows "not available in this build".

**Map rule (tested):** from every enemy and player spawn, a 16×16 tank must be able to reach the base treating
bricks as open and steel/water as walls (`levels::tests::every_spawn_can_reach_the_base`). Watch for: a water gap
that exits straight into steel, and 1-cell (8-unit) open bands between obstacle rows — a tank needs 2 cells.
**Centre lane (tested):** every column over the base ring (map cols 11–14) has steel between the top spawns and the
ring, usually a 4-cell bar a few rows above it, so no enemy can shoot the base straight down the centre; the base is
open only from the sides (`levels::tests::centre_spawn_has_no_free_shot_at_the_base`). Leave 2 open cells between the
bar and any steel/water row above it (tank height).
**Side lanes (tested):** without shooting a brick, every way from a top spawn down to the bottom rows (a tank at cell
row 2 or lower) is at least 30 cells long, straight down being 22, so a scout can't race down an open side lane
(`no_quick_run_from_a_top_spawn_to_the_backline`); most stages put a brick row across the lower half for this.
**No sealed centre (tested):** with map cols 11–14 walled in the lower half, both corner spawns still get round to a
player spawn, so a player parked over the base can't hold every enemy off (`a_tank_in_the_centre_lane_cannot_seal_the_field`).
**No pockets (tested):** walking out of any dead end (bricks open, spawns exempt) reaches a junction within 4 cells,
so enemies don't dither in a long blind lane (`no_dead_end_pockets`).

## Controls
| | Controller | Keyboard P1 | Keyboard P2 |
|---|---|---|---|
| Move | D-pad / left stick | WASD | Arrows |
| Fire / confirm | A (also X, RB, RT fire) | Space / F | Enter / Right Ctrl / Num 0 |
| Back / leave slot | B | Backspace | Backspace |
| Pause | Start | Esc | Esc |

A host that feeds its own gamepads can tag one as a **TV remote** (`TvRemote`): it has no Start, so Back also toggles
pause in a stage. With `Platform::back_exits_title`, Back on the title with nobody joined quits.
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
| SOUND FX | 0–100 % in 10s | 80 % | sound effects (`volume=` in the file); 0 skips playback entirely |
| MUSIC | 0–100 % in 10s | 60 % | music track volume, changes live; 0 stops the music |
| SCREEN SHAKE | ON/OFF | ON | comfort on a big TV |
| SHOW FPS | ON/OFF | OFF | smoothed FPS, top-right, every screen |
| FRIENDLY FIRE | ON/OFF | ON | ON = classic (players paralyze each other); OFF = player bullets pass through the other player |
| EASY MODE | ON/OFF | OFF | ON = losing a tank (or spending level-4 armour) costs one star, not all of them (City Run districts still start at 0); 2 more spare tanks at the start of a classic run or district try; a 6 s respawn shield instead of 3 s. Stages and districts cleared in easy mode open only for easy mode (`easy_classic=` / `easy_unlocked=` in `city.txt` and the cloud copy; an easy run may start anything either frontier opened). Such runs skip the world board and earn no achievements (the local hi score counts); the row shows `ON - NOT RANKED`. Turning it on opens a popup that explains this: A turns it on, B keeps it off. Saved as `easy_mode` (older files' `keep_power` is read as it) |
| VIBRATION | ON/OFF | ON | rumble on the slot's own pad: own tank destroyed (a sad, falling three-pulse), enemy hit (short), kill (medium), mini-boss / boss kill (strong), paralyzed (medium), base lost (everyone, long), power-up (tick) |

A host that owns the window (`Platform::host_window`) hides VSYNC and FULLSCREEN.

Leaving the game (Home, another app, TV off) pauses a live stage, holds the music and cuts sound effects still ringing; on return the pause menu waits.

Controller disconnect → game pauses with a note; any unassigned controller pressing A takes over the slot.

## City Run (`game/city.rs`, `city_map.rs`, `levels/city/NN.txt`)
Title → CITY RUN → district map (10 dots on a pixel-art city, dotted roads). Cleared = gold, next = white,
later = locked. Left/right picks among unlocked, A starts, B back. Unlocks saved to
`%APPDATA%\BattleTanks\city.txt` (`unlocked=N`, classic stages `classic=N`). A district opens only by clearing the one
before it (no gaps; same rule as classic stages). A host with a cloud copy (`Platform::cloud_load` / `cloud_save`,
none on the exe) syncs it at start and after each new unlock: the higher count wins on both sides, and a cloud that
can't be read is never written.

- A district is 15 screens tall (district 1) up to ~24 (district 10). Only one field-sized slice is visible: the
  `View`. It follows the lead player (kept at 55 % height) but keeps the trailing tank in view with 16 units (one
  tank) of room below it, so a tank in a notch can still back out, and never scrolls back down. Tanks can't leave the view
  (one the view left half outside may still move along or back in); bullets pop at its edges. The view rises at most
  96 units/s (`VIEW_CATCH_UP`, twice a tank), so after a mini-boss or boss lets go it glides up instead of jumping.
- **District track** (HUD right panel, in place of the enemy reserve): a bar from the start to the boss icon on top,
  gold up to the top of the view (how much of the district you've seen), a 1P dot left of it and a 2P dot right of it
  at each tank's height.
- Every district starts fresh: 3 spare tanks each (one more than classic), star level 0 (upgrades don't carry
  over). Score starts from 0 on every try (per district, like the clear time).
- **Stashes**: brick boxes holding a power-up from the start (`SS` star, `LL` extra tank in the map file; the
  half-row mirroring makes a pair, one per side). Shoot in to collect. Every district has a star pair low down
  and a tank pair further up; a new enemy drop never replaces a stash (tested: `city_map::tests`).
  No stash of any kind on a district's first screen (tested).
- **Secret stashes** (`??` in the map file, 3 per district): a power-up (star, tank, helmet, grenade or clock, fixed
  per district) buried in the core of a solid 4x4 brick block that looks like any other. The core is `Tile::Secret`:
  it blocks like brick, but enemy shells only pop on it (they still chip the bricks around it), so only a player can
  open a secret. Breaking into the core makes the power-up pop out (sound + debris); it then stays put like a stash.
  Each district also has 9 plain look-alike 4x4 brick blocks (3 per secret), so a lone brick block is no tell.
  All players out → game over → back to the map to retry.
- No base. Enemies hunt the nearest player. Placed enemies wake (sparkle) once fully in view; every 5th placed
  enemy carries a power-up. Reinforcements drop in at the top every 7 s − 0.35·district − 1 s in 2P (≥ 2.5 s),
  capped like classic (4 / 6 on screen), until the boss appears.
  A drop spot must be free of tanks and of placed enemies not yet awake, and one a tank can drive to from the start
  (gates open; sealed pockets in the street art are skipped, tested).
- **Bastions**: a full-width fortress wall (`=`, unbreakable even at star 3) with turrets and an orange gate (`G`).
  When every turret of a bastion is destroyed the gate opens; a dead turret's spot becomes wall.
- **Turrets** never move, aim up or down at the nearest player (never sideways into their own wall),
  HP 2 + d/3, 500 pts. **Mini-bosses** 24×24, HP 8 + d, 2 bullets,
  1000 pts, drop a power-up; **boss** 32×32 at the map top, HP 20 + 3d, 3 bullets, 5000 pts. While a mini-boss or
  the boss is alive the camera holds still. Killing the boss clears the district.
- Power-ups: no shovel; grenade only destroys ordinary tanks. Drops land on open ground in view.
- Difficulty per district: enemy mix (`mix` line), more bastions (3→8) and turrets per wall (2→6), more minis
  (2→5), faster turret/boss fire.
- Map rule (tested): from the start, every mini-boss and the boss is reachable with gates open, and closing any one
  gate cuts the boss off (`city_map::tests`).
- Map validator (`city_gen::sound`, tested on the committed files, and the generator redraws any band that fails it):
  no reachable spot whose only way up means backing down more than 2 cells (the view's room), and every turret has
  no steel or wall in the 6 rows in front of it across its columns and one either side (bricks and water are fine)
  plus a reachable straight shot from below within one screen.
- **Looks follow the name.** Maps are drawn by `city_gen` (test-only, `cargo test write_city_districts -- --ignored`)
  from per-theme brushes: Outskirts fields, hedgerows, farms and ponds; Market Row stalls, arcades and fountain squares;
  Canal Ward canals with bridges, terraces and locks; Old Town crooked house blocks, squares, a church and the old wall;
  Ironworks halls, pipe runs, scrap yards and cooling ponds; Frost Park ice fields, frozen ponds and groves; Harbour
  basins with piers, containers and cranes; Rail Yards tracks with wagons, platforms and engine sheds; Citadel Gate
  trenches, bunkers and tank traps; The Citadel steel courtyards, brick mazes, the moat and pillar halls. Each district
  keeps its height, turrets, minis and placed enemies (the world board's district table depends on them). No 8-row
  stretch of street appears twice anywhere in City Run (tested).

## Music (`music.rs`, ADR 0007)
All tracks are normalised to −16 LUFS (linear gain from the MP3 master, see Operational lessons for the
command). One track at a time, chosen by screen: title + settings `menu_music`, City Run map `menu_music_alternative`,
results `victory`, game over `loss` (2, rotating). In a stage the screen picks a **pool** and the
music walks through it: when a track ends the next one in the pool starts (no track twice in a row unless the
pool has one). Classic stages 1–10 slow (4 tracks), 11–20 medium (5), 21–30 fast (6), by map so laps restart
slow; City Run districts 1–3 slow + medium 1–2, 4–7 medium + fast 1–2, 8–10 fast. The stage number picks the opening
track, so consecutive stages start differently. In City Run a live mini-boss switches to the `hardfight`
pair, the boss to the `bossfight` pair, and when it dies the stage pool goes on with the track after the one the
fight interrupted (not back to its opener). The pause menu pauses the music. Jingles (stage start, game over)
play over it.

## Achievements (`achieve/`, `ui/achievements.rs`, ADR 0017; list and rules in `achievements.md`)
Only on hosts with `Platform::achieve` (Android: Play Games); the exe has none (no tracking, no menu).
- 56 entries in six groups. Counters (kills, HEAVY kills, bosses, turrets, cancelled shells) show `n / target`;
  hidden ones show `???` until unlocked. Only this device's players count: on a couch both slots, over LAN only
  this machine's. A streak (kills since the last lost tank) and "in one run" counts end with the run: a classic run
  from the title, or one district try. "Without losing a tank" needs the player still in at the clear.
  Secret stashes count for the device whoever broke in.
- Kept in `achievements.txt` (`done=<keys>`, `<counter>=<steps>`) and in the cloud copy after `unlocked=N` (union
  of unlocks, max of counters). Saved and told to the host off the stage (results, game over, title), so a stage's
  unlocks show up there, not mid-fight; the host also hears everything once at start.
- **Badges**: every entry has its own 32x32 pixel badge (painted at startup, no files): a plate in its group's
  colour (classic olive, duo teal, City Run slate, City Run extras violet, combat crimson, fun amber) with a gold rim,
  an emblem that tells the story (stage flags, a scarecrow for the Outskirts, the burning Citadel, popcorn for the
  Backseat Driver...) and a number tag where it has one (10, 1K, 100K). Locked badges are grey; hidden ones a "?".
- **In a stage**, each unlock sends a pixel-art gold star in from the screen's side onto the dark margin beside the
  field (right first, then left; more pairs a little lower, then higher); it swells, then bursts into gold pixel cubes
  with a chime. The run summary still comes after the stage.
- Title → ACHIEVEMENTS: points earned (of 950) and `N / 56`; a tab per group with its count (CLASSIC, DUO, CITY,
  CITY+, COMBAT, FUN); 7 rows per page, each with the badge, name, description and points (gold gem). Unlocked rows
  are gold with a gold edge; counters still running show `n / target` and a progress bar; hidden ones `???`.
  Left/right tab, up/down scroll (scrollbar on the right), B back.
- **NEW ACHIEVEMENTS** (its own screen, before the world board): after a classic game over, or a City Run district's
  results or game over, when that run unlocked something. The rows arrive one by one (slide in, gold flash, badge
  pop, chime), the points total counts up under a twinkling title; A shows all at once, then moves on. Not shown in
  LAN sessions (both machines must change screens on the same tick); the list has them.

## LAN co-op (`net/`, `ui/lan.rs`, ADR 0014)
Title → LAN CO-OP (hosts whose `Platform::lan` is set). One player picks HOST GAME (the screen says "HOSTING AS
<name>"), the other sees that name in the list (JOIN <NAME>) and picks it. Both machines must run the same version
(the list says which version a host runs). Once linked both go to the title with nobody joined; each player joins
with A on their own device and is shown by machine name. Then it is the normal game for two: classic or City Run,
same rules, each machine draws it and plays its own sound and rumble.
- Linked title: CLASSIC, CITY RUN, LEAVE LAN (no HI SCORES / SETTINGS / EXIT). Pause menu: EXIT GAME becomes
  LEAVE LAN. Leaving ends it for both. The host's FRIENDLY FIRE, EASY MODE, open stages and districts apply; the
  joiner's own come back afterwards.
- Unlocks over LAN (ADR 0018): the host's move as always. A stage or district cleared together also counts for the
  joiner, but only from their own frontier: own open stage 3, clear 5 with the host → nothing opens for them (3 and 4
  stay locked); own 5, clear 5, 6, 7 in a row → own 8. Each side saves and syncs only its own progress.
- LAN runs don't go to the world board (the local hi score still counts).
- One machine going to the background pauses the stage on both. A slow or silent partner freezes both screens with
  "waiting for <name>..."; after 45 s the session ends and the remaining player goes on alone (the partner's slot is
  a lost controller: pause, any local pad takes it over). A mismatch found off a stage ends it with "lost sync".
- Input delay picked per machine from the measured round trip (ADR 0015): starts at 3 ticks (50 ms), usually settles
  at 2 (33 ms) on home Wi-Fi, up to 5 on a bad link; the linked title shows the round trip and the delay.

## Known gaps / deliberate omissions
- LAN: two players only, same network, same version; a phone acting as a hotspot host may not be found (the beacon
  guesses a /24 broadcast); no reconnect after a drop.
- The world board can't stop a careful cheat who extracts the key and fakes a plausible stage log (open source).
- HI SCORES shows the top 10 only; the website shows the top 100 of every board.
- City Run keeps no local best time per district; only the world board has it.
- Grenade-killed spawning enemies leave their sparkle to finish (cosmetic).
- Keyboard layouts share Esc and Backspace; both act for whoever is joined on the keyboard.
- Variable timestep (dt clamped 0.05 s) rather than fixed; fine at 60 fps vsync, revisit if behaviour drifts
  at very low frame rates.
- No online play, no AI teammate, no level editor (maps are plain text). Network use: the world board (sending runs,
  HI SCORES) and LAN co-op on the local network.
- City Run: no boss HP bar (bosses fade to grey as they take damage); the view never scrolls back down.

## Operational lessons
| Symptom | Cause | Guard |
|---|---|---|
| `bevy_a11y = "^0.20.1"` not found | stale local crates.io index cache | delete `Cargo.lock`, `cargo generate-lockfile` |
| Startup panic "Resource does not exist" | `OnEnter(Title)` runs before `PreStartup` commands apply | build `Gfx` in the plugin fn, not a startup system |
| One Esc press joined both keyboard slots | Esc is "start" on both layouts | join only on confirm, never on start |
| Armor enemy looked like player 2 | green tint shared with 2P | enemy palette never uses gold/green |
| ~1000 fps on the owner's PC despite `AutoVsync` | VSync not honoured (driver override / streaming setup) | own FPS limiter in Settings, default 60 |
| New stages walled spawns off from the base | bridge exits into steel; 1-cell open bands between blob rows | reachability test over all stages |
| Co-op stuck: P2 in a pocket beside a pond (only exit under it), the view pinned to P2, P1 couldn't go on | view stopped exactly at the trailing tank's bottom, so backing down was impossible | view keeps 16 units below the trailing tank; `city_gen::sound` rejects pockets needing a longer back-down |
| A turret no shell could reach (steel straight under it, district 4) | street steel landed in the turret's column below the wall | `city_gen::sound`: steel-free rows in front of every turret + a clear shot |
| City map had a river nobody could cross | two stacked bridges not overlapping (tank needs a 2-cell column through both) | `city_map` reachability test |
| Map markers doubled (two bosses) | half rows are mirrored | draw single markers centred on cols 11–14 or as full 26-char rows |
| Enemy killed the base from the centre spawn in 3 shots (stage 1 + 13 more) | open centre column, ring only 2 rows thick | centre-lane cover test over all stages |
| Base needed a full-time guard: most stages had only 2 brick cells between the centre lane and the ring (owner) | brick roof falls in a few shots | steel bar over the ring on every stage, tested |
| A SCOUT rushed in as stage 1's first enemy | roster shuffle can put any kind first | opening wave = 3 weakest, tested |
| A power-up dropped inside steel, another in water (owner) | the classic drop spot ignored terrain | `powerup::classic_spot`: open ground reachable from player 1's spawn, tested on every stage |
| STAGE CLEAR / GAME OVER hard to read in a stage (owner) | brick block letters straight over a dimmed, busy field | banner on a dark plate with a steel rim |
| Stage 6: enemies stuck in the side lanes (owner) | the lanes ran from the top spawns into a corner between steel and water, 17 cells deep | ponds moved inward; `no_dead_end_pockets` |
| Stage 17: a player in the centre lane over the base held off every enemy (owner) | steel pickets with 1-cell gaps; the only way down was the 2-cell gap over the base | wider gaps; `a_tank_in_the_centre_lane_cannot_seal_the_field` |
| 1P: scouts raced down open side lanes to the base (owner; stages 1, 4, 5, 7, 10, 12, 16, 20-22, 24) | straight or 2-4-sidestep brick-free ways from the top spawns to the bottom | brick rows across the lower half; `no_quick_run_from_a_top_spawn_to_the_backline` |
| Smoke runs blasted sound in the room | audio always on | audio plugin skips when `BT_SHOT` / `BT_MUTE` is set |
| 10 fps on a TV-class GPU (Mali-G57), CPU idle | Bevy's default 4x MSAA + bloom passes at 1080p, each ~halving fps | `Msaa::Off` everywhere (sprites are pre-anti-aliased); bloom is `Platform::bloom`, off on such hosts: 56 fps |
| Looping music would hold ~55 MB per song | rodio `repeat_infinite` buffers every decoded sample | play once (DESPAWN), respawn when it ends |
| New track failed `every_track_decodes` (`UnrecognizedFormat`) | MP3 cover art became a Theora video stream in the .ogg | encode with `ffmpeg -i x.mp3 -map 0:a -vn -map_metadata -1 -af volume=<-16 − measured LUFS>dB -ar 44100 -c:a libvorbis -q:a 1 x.ogg` (measure with `-af loudnorm=print_format=json -f null -`) |
| Stage music restarted from its pool's opener after a mini-boss died (owner) | a pool change started fresh, forgetting the track the fight cut in on | `music::handover` keeps it in `Music.before`, the stage pool goes on after it |
| City Run enemy spun in place at the bottom of the view until culled | the scrolling view left it half outside, and the edge test blocked every move | the edges only stop a move further out (`tank::leaves_view`) |
| City Run reinforcement stuck in a bastion wall, spinning | dropped into a turret's notch (open ground until the turret wakes), then the turret woke on top of it | drop spots skip pending spawns |
| Turrets easy in co-op: one player beside the turret, the other shooting it from below | turret aimed along the longer axis at the nearest player, i.e. sideways into its own wall | `enemy::turret_dir`: up or down only |
| City Run reinforcement stuck for good at district 5's left edge above a bastion (owner) | dropped into a pocket sealed by steel and the wall (open ground, so the drop check passed) | drops only on cells reachable from the start (`city::Reach`); placed enemies too (tested) |
| Screen jumped when a City Run boss died | the camera lock let go and the view snapped to its target in one frame | `rules::VIEW_CATCH_UP` caps how fast the view rises |
| HUD jumped sideways when a player went out and "A: BORROW A TANK" showed (owner) | side panels grew from their content width (`flex_basis: auto`), so the longer line widened one side | panels `flex_basis: 0`: equal split whatever the text |
| `adb pull /sdcard/...` "no such file" from Git Bash | MSYS rewrites the remote path to `C:/Program Files/Git/sdcard/...` | prefix `MSYS_NO_PATHCONV=1` |
