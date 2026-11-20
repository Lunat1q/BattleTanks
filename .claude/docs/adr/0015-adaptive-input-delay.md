# 0015 — LAN input delay picked from the measured round trip

Status: accepted (2026-11-19); supersedes the fixed 3-tick delay of ADR 0014

## Context
A fixed 50 ms delay is safe on most Wi-Fi but more than a good network needs, and too little on a bad one (stalls).

## Decision
- Each datagram carries a clock stamp and the echo of the peer's latest one (with how long it was held), so each side
  measures the round trip continuously (`lockstep::Lockstep::receive`).
- Each side picks its own delay: the 95th percentile round trip of the last ~3 s, half of it for the one-way trip,
  plus one tick for the two machines' ticks not lining up, 1-5 ticks (`pick_delay`). Retuned every second: up at once,
  down by one only after 5 s of lower readings.
- No agreement is needed: a side's delay only decides how early its own input goes out; the peer waits for whatever
  arrives. Raising fills the gap with held input, lowering skips packing until the simulation catches up. Both start
  at 3 ticks, which both assume for the first ticks.
- The linked title shows the round trip and the delay. Datagram format changed: `PROTO` 2.

## Consequences
- Home Wi-Fi typically settles at 2 ticks (33 ms); a spiky network climbs to 4-5 instead of stuttering.
- Spikes above the 95th percentile still stall for a moment.
