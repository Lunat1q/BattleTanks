# 0002 — All art and sound generated at startup

Status: accepted (2026-11-18)

## Context
The game must look better than the 8-bit original, be fully original (ADR 0003) and launch from Steam
without a working-directory/asset-folder dance.

## Decision
Textures are painted at startup by a small anti-aliased SDF rasteriser (`gfx/canvas.rs`) at 8 px per unit and
kept in a `Gfx` resource built when the plugin is added. Tanks are drawn grey and tinted per team via
`Sprite::color`. Bullets/flashes use HDR colours with camera bloom. Sounds are synthesised into in-memory WAVs.

## Alternatives
- Hand-drawn PNG/OGG in `assets/`: needs an artist, a cwd-correct launch, and still risks looking derivative.
- Embedded asset bytes: same art problem.

## Consequences
- The release exe is self-contained. Startup paints ~1 M pixels (well under a second in release).
- Art changes are code changes; the smoke screenshot hook (`BT_SHOT`) is how to check them.
- Ceiling: no animation beyond frame swaps/particles; real sprite sheets would need an asset pipeline.
