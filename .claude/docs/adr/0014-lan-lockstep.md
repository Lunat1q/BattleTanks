# 0014 — LAN co-op: lockstep inputs, host snapshot when the hashes differ

Status: accepted (2026-11-19)

## Context
Two players on two devices (phones, or a phone and a PC) on the same Wi-Fi should play one game, each seeing it on
their own screen. The game was built for one machine: variable timestep, rendering and simulation in the same frame.

## Decision
- **Lockstep.** Each tick both machines send their player's packed `Intent` (2 bytes, `net::wire`) for tick + 3;
  a tick runs only once both inputs are in, with a fixed `dt` of 1/60 s. Inputs go over UDP, every datagram
  repeating the not yet acknowledged ones, so a lost packet costs nothing. TCP carries the hello (version, seed, the
  host's FRIENDLY FIRE / KEEP POWER and City Run unlocks), snapshots and the goodbye.
- **The driver replaces `Update` in the main schedule** (`net::Driver`). Offline it runs `Update` once per frame, so
  nothing changes for local play; linked it runs `StateTransition` + `Update` once per tick. No system had to move.
- **Determinism rules:** gameplay randomness only from the gameplay `Rng` (particles have their own); sim loops whose
  outcome depends on order walk entities by `NetId` (spawn order), because Bevy's archetype order differs between
  machines (gfx adds sprites on different frames). A headless test runs two worlds, one disturbed the way rendering
  disturbs it, and compares hashes every 30 ticks (`net::tests`).
- **Repair, not prevention only.** Every 30 ticks both hash the stage (`net::snap`, screen + session + all sim
  state). If the hashes differ, the host flags one of its upcoming inputs; after that tick the host takes a
  snapshot, loads it itself and sends it, and the joiner waits for it and loads it. Both worlds then respawn every
  sim entity in the same order. Off the stage a mismatch ends the session ("lost sync").
- **Discovery:** the host broadcasts `BTANKS <proto> <version> <port> <name>` on UDP 47474 once a second (all-hosts,
  the /24 broadcast, loopback) until someone joins. The name comes from `Platform::lan`.
- LAN runs skip the world board (name entry waits on a network answer, which the machines would get at different ticks);
  HI SCORES and SETTINGS are hidden while linked.

## Alternatives
- Streaming the host's screen as video: no determinism work, but a consent dialog on every phone session, lag and no
  sound for player 2, and a Play foreground-service declaration.
- Host-authoritative state replication: robust, but every component and screen serialized every tick.
- Rollback (GGPO style): hides the input delay, but needs a snapshot every tick plus re-simulation.

## Consequences
- 50 ms input delay for both players (`lockstep::DELAY`, a tuning knob).
- Any new gameplay randomness, timer or order-dependent loop must follow the rules above, or the determinism test
  fails (or, if it slips past the test, the session resyncs now and then).
- A slow or paused partner stalls both screens ("waiting for ..."); 45 s of silence ends the session, and the player
  left continues alone (the partner's slot shows as a lost controller).
- Both machines must run the same version.
