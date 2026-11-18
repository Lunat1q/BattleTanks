# 0006 — Android TV build next to the Windows exe

Status: proposed (2026-11-18)

## Context
The owner wants the game on Google Play for Android TV, alongside the Windows exe. Bevy 0.20 targets Android
(`android-game-activity`), but its gamepad backend, gilrs, has no Android backend (`gilrs-core` 0.6.8 ships only
windows/linux/macos/wasm plus a dummy), so controllers would be dead. Android itself exposes Xbox, PlayStation
and generic HID pads (USB and Bluetooth) as `InputDevice`s with `KEYCODE_BUTTON_*` and `AXIS_*` events. The 2026
Steam Controller ("Triton") is the exception: Android only sees its mouse/keyboard "lizard mode". The owner wrote a
driver for it in moonlight-android PR #623 (BLE, 2.4 GHz puck, USB), with the protocol ported from SDL's Triton
driver.

## Decision
- **One codebase, two targets.** The game moves into `src/lib.rs` (`pub fn run()`); `main.rs` (Windows) and an
  Android `#[bevy_main]` both call it. Game logic, art and sound do not fork.
- **The Java side owns the controllers; the Rust side pretends to be gilrs.** A small Gradle project in `android/`
  has a `GameActivity` subclass that intercepts and consumes every gamepad, D-pad and TV-remote key and motion
  event before winit sees it. It also runs the Steam Controller driver. It forwards one compact event stream over
  JNI: connect, disconnect, button, axis, plus a stable pad id. On Android, `src/android.rs` turns that stream into
  Bevy's `GamepadConnectionEvent` and `RawGamepadEvent`, just as `bevy_gilrs` does on Windows. `input.rs`,
  `Device::Pad(Entity)` and the rest of the game stay unchanged. Rumble goes the other way:
  `GamepadRumbleRequest` → JNI → `VibratorManager`, or the Triton driver.
- **The TV remote counts as one more pad** (D-pad, centre = A, Back = B), so every menu, and 1P play, works
  without a controller. Play's TV review requires this.
- **Licences:** no third-party GPL code enters this repo. Moonlight is GPL-3.0, which forbids the extra
  restrictions in our PolyForm Noncommercial licence, so copying its code would force the whole game to GPLv3. The
  Triton driver files are the owner's own work from PR #623, so the owner can relicense them here. Protocol facts
  are checked against SDL (zlib, compatible). For Xbox pads we use Android's native `InputDevice` support and do
  not port Moonlight's Xbox USB drivers, which were written by other authors.

## Alternatives
- Wait for gilrs or winit to support Android gamepads: no timeline, and neither would handle the Triton.
- Read input through `android-activity` in Rust only: winit owns that event loop and drops joystick motion, and the
  BLE GATT and USB host APIs are Java-only anyway.
- Ship GPLv3 and copy Moonlight's whole input stack: changes the game's licence. Rejected.
- Stream only (Moonlight on the TV): already works today, but needs the PC, so it can't go on the Play Store.

## Consequences
- New tooling: Android SDK and NDK r28+, `cargo-ndk`, Gradle. One new crate on Android only: `jni`.
- Two input stacks to keep equivalent. The Rust side stays tiny because it only emits Bevy's own raw events.
- Windows keeps Steam Input for the Steam Controller. We add no Windows driver.
- Ceiling: no gyro or right-trackpad features. The left trackpad drives the stick as in the PR. Add more when a
  mode needs it.
