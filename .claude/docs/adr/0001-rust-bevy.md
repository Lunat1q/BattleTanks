# 0001 — Rust + Bevy 0.20, no extra crates

Status: accepted (2026-11-18)

## Context
A 2D couch game for a Windows PC streamed to a TV. Owner asked for Bevy. Needs gamepads, fullscreen, audio,
2D sprites, UI text — and should ship as one exe for a Steam non-Steam shortcut.

## Decision
Rust 2024 + Bevy 0.20.1 with `default-features = false` and only `2d`, `ui`, `bevy_audio`, `wav`, `png`.
No other dependencies: own xorshift RNG, own WAV writer, own SDF painter.

## Alternatives
- Bevy default features: pulls 3D/PBR/glTF; slower builds for nothing we use.
- `rand`, `bevy_kira_audio`, `leafwing-input-manager`: each replaces < 100 lines we can own; skipped.

## Consequences
- Bevy 0.20 is newer than most model training data; API is checked against the registry source (CLAUDE.md).
- Bevy upgrades are breaking each release: upgrade deliberately, one version at a time.
