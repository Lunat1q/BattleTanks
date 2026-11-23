# 0019 — VERSUS: a two-player duel, with optional NPC armies

Status: accepted (2026-11-23, owner; not built yet, plan M18)

## Context
Every mode is co-op. The owner wants a couch duel: each player defends a base and attacks the other's. Optionally
each player also brings friendly NPC tanks, bought from a points budget before the duel, which spawn over time and
fight for their side. The map is either picked from a pre-generated set or generated at random.

## Decision

**Field and map**
- The classic 208 x 208 field. Player 1's base sits bottom centre as now; player 2's is its 180° rotation at top
  centre, each with the usual brick ring.
- Maps are **point-symmetric** (cell `(c, r)` = cell `(25 - c, 25 - r)`), so both sides get the same map. A map file
  holds only the bottom 13 rows; the top half is the bottom half rotated.
- **Spawns:** each player spawns beside their own base, as in classic. NPCs spawn in the two corners of their side's
  back row.
- **Map rules**, tested on every map like the classic ones:
  - every spawn can reach the other base;
  - steel covers each base from straight shots down the centre lane;
  - no dead-end pockets;
  - no free shot from a spawn into the other base.
- **Pre-generated maps:** `levels/versus/NN.txt`, about 12 of them. A generator (`versus_gen`, like `city_gen`) draws
  them from brushes, then redraws any map the rules reject.
- **RANDOM** runs the same generator at runtime from the session seed, so both LAN machines build the same map. That
  means the generator is runtime code, not test-only.

**Sides**
- `Team::Npc(side)` joins `Team::Player(i)` and `Team::Enemy`, and `Team::side()` gives 0 or 1 (`Enemy` has no side).
- Hits:
  - Different sides always hit each other. Player against player destroys the tank; it no longer just paralyzes.
  - The same side never hits itself. Shells from your own NPCs pass through you, and yours pass through them. So
    FRIENDLY FIRE doesn't apply in VERSUS.
- Shells cancel each other only across sides.
- A base takes damage only from the other side, so nobody can lose by shooting their own base, NPCs included.
- Looks: NPCs use the enemy tank models, tinted a darker shade of their player's colour (gold, green).

**NPC armies** (optional; OFF = a plain duel)
- **Budget:** OFF / 10 / 20 / 30 points per player, default 20.
- **Costs** (`rules.rs`): LIGHT 1, SCOUT 2, GUNNER 3, HEAVY 5.
- **Draft:** before the duel each player buys their army on their own card with their own pad.
  - Up/down picks a type; left/right changes its count; the card shows the points left.
  - AUTO fills a mixed army.
  - The duel starts once both have pressed A.
- **Spawning:**
  - Each side gets one NPC every 4 s, with at most 4 of its NPCs alive at once, until its army has all spawned.
  - The order is the army shuffled by the gameplay `Rng`, so it stays deterministic over LAN.
  - Every 4th NPC of a side flashes and drops a power-up when killed, as in classic.
- **NPC AI:** `choose_dir` with new targets.
  - 60 % head for the other base; 40 % head for the nearest hostile tank (the other player or their NPCs).
  - Wandering drifts towards the other side.
  - Fire timing is the enemies' own. NPCs can't hurt their own base or their own side, so they need no aiming rule.

**Players, power-ups, end**
- 3 tanks each (EASY MODE gives both 2 more), with a shield on respawn.
- Power-ups come from NPC drops. With armies OFF, one appears in the middle band every 20 s.
- In VERSUS, several power-ups act on the opponent:
  - the clock freezes the other side, player and NPCs, for 5 s;
  - the grenade destroys the other side's NPCs on the field;
  - the shovel turns your own base ring to steel.
- **Win:** the round goes to whoever destroys the other base, or knocks the other player out of tanks. Both on the
  same tick is a draw.
- A match is 1, 3 or 5 rounds (default 3). The results screen shows round wins and kills by type.
- Rematch keeps the setup; RANDOM draws a new map each round.

**Flow**
- Title → PLAY shows a VERSUS card next to CLASSIC and CITY RUN (owner, 2026-11-23: keeps the title menu short). It
  needs two players joined, and is also shown when linked over LAN.
- PLAY → VERSUS → `Screen::VersusSetup`:
  - MAP: RANDOM, or tiles of the pre-generated maps with mini-maps (`stage_map`);
  - NPC ARMY: OFF / 10 / 20 / 30;
  - ROUNDS: 1 / 3 / 5.
- Then `Screen::Draft` when armies are on, then the stage (`Mode::Versus`), then `Screen::VersusResults`.
- From results: A plays the next round or a rematch; B goes to the title.
- No world board and no achievements (no fair score to rank). The local hi score is not touched.

**Code** (fits ADR 0005: VERSUS shares the stage, like City Run)
- `game/mod.rs`: `Mode::Versus`, `Team::Npc`, `Base(side)`.
- `combat.rs`: `can_hit` / `opposing` by side, bases hit only by the other side.
- `game/versus.rs`: army, spawn cadence, NPC targets, round end.
- `versus_map.rs` + `versus_gen.rs`: parse, generate, validate.
- `ui/versus.rs`: setup and draft screens; HUD and results.
- gfx: side tints, top base.
- `net/snap` + `Hello`: the new state and the versus setup.
- `music::pick`: a pool for VERSUS.

## Consequences
- Pure rules stay testable: costs, budget, draft, side hits, map symmetry and the map rules.
- `net::tests` gains a versus world, so LAN versus is checked for drift like the other modes.
- Player-vs-player kills are new (classic only paralyzes), so the Rumble, Fx and HUD paths need checking.
- The generator moves from test-only to runtime; its validator must stay fast enough for RANDOM (redraws included).

## Owner's answers (2026-11-23)
Draft with AUTO; bases immune to their own side; no round time limit (add one only if couch play shows stalls); the
VERSUS clock, grenade and shovel as above.
