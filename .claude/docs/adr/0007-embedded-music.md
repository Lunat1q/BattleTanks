# 0007 — Owner-made music, embedded as Ogg Vorbis

Status: accepted (2026-11-18). Amends 0002 for music only; sound effects stay synthesised.

## Context
The owner composed menu, battle (slow/medium/fast), mini-boss, boss, victory and loss tracks (MP3, ~27 MB,
128-180 kbps). The game ships as one exe with no asset folder.

## Decision
Re-encode to Ogg Vorbis q1 (~70 kbps, 44.1 kHz stereo, ~12 MB total) with ffmpeg, commit the `.ogg` files in
`assets/music/`, and bake them in with `include_bytes!` (`src/music.rs`). Decoding uses Bevy's `vorbis` feature
(rodio + lewton, pure Rust). The MP3 masters stay on the owner's PC (gitignored).
Tracks play with `PlaybackSettings::DESPAWN` and are respawned when they end, not `LOOP`: rodio's infinite
repeat buffers the whole decoded song (~55 MB for 2.5 min).

## Alternatives
- Ship MP3 (`mp3` feature): bigger at the same quality.
- Loose files next to the exe: breaks the single-file Steam launch.
- Opus: smaller, but Bevy 0.20 has no Opus decoder.

## Consequences
- The exe grows by ~12 MB.
- New track = add the `.ogg` (`ffmpeg -i x.mp3 -map_metadata -1 -ar 44100 -c:a libvorbis -q:a 1 x.ogg`),
  a `Track` variant and a line in `music::pick`.
- A one-frame gap when a track restarts; acceptable for long tracks.
