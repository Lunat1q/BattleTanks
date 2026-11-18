# 0004 — Exe icon via a hand-built .res, window icon via winit

Status: accepted (2026-11-18)

## Context
Steam's non-Steam shortcut and Explorer show the exe's embedded icon; the taskbar/Alt-Tab shows the window icon.
Bevy has no API for either. ADR 0002 keeps art procedural.

## Decision
- The icon is painted by `gfx/icon.rs` from the game's own tank art. An ignored test writes `assets/icon.ico`
  (32-bit BMP entries, 16–256 px) and `assets/icon.png`; both are committed.
- `build.rs` converts the ICO into a Win32 `.res` (RT_ICON + RT_GROUP_ICON) in `OUT_DIR` and passes it to the
  MSVC linker with `cargo:rustc-link-arg-bins`. No build-dependencies, no `rc.exe` on PATH needed.
- At runtime a main-thread system sets the window icon via `bevy::winit::WINIT_WINDOWS` and `winit::window::Icon`
  (`winit` 0.30 added as a direct dependency: same version Bevy builds, nothing extra compiled).

## Alternatives
- `winresource` / `embed-resource` crates: need the Windows SDK `rc.exe` or `windres`, plus build deps.
- Icon only at runtime: Steam/Explorer would still show the generic exe icon.

## Consequences
- Non-MSVC targets skip the embed (build.rs checks `CARGO_CFG_TARGET_ENV`).
- Changing the icon art requires re-running the ignored test; `build.rs` reruns when `assets/icon.ico` changes.
