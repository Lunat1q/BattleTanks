//! Android controllers (ADR 0006). gilrs has no Android backend, so the Java side (`PadHub.java`) takes every
//! gamepad, TV-remote and Steam Controller event before winit sees it and calls `nativePad` with raw Android
//! codes. This module plays the part `bevy_gilrs` plays on Windows: it turns them into Bevy's own
//! `GamepadConnectionEvent` / `RawGamepadEvent`, so `input.rs` and the game don't know the difference.
//! Rumble goes the other way: Java polls `nativePollRumble` every frame.

// Built on Windows only for the mapping tests; the runtime half is Android-only.
#![cfg_attr(not(target_os = "android"), allow(dead_code))]

use crate::input::TvRemote;
use bevy::input::gamepad::{
    GamepadAxis, GamepadButton, GamepadConnection, GamepadConnectionEvent, GamepadRumbleRequest, RawGamepadAxisChangedEvent,
    RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use std::sync::Mutex;

// Event kinds; must match PadHub.java.
const CONNECT: i32 = 0;
const DISCONNECT: i32 = 1;
const KEY: i32 = 2;
const AXIS: i32 = 3;
/// Set in a CONNECT code's top bit: the device is a TV remote (D-pad, OK, Back only).
const REMOTE_FLAG: i32 = 1 << 31;

/// Android `KeyEvent.KEYCODE_*` → Bevy button. Remotes send D-pad/OK/Back; pads the `BUTTON_*` codes.
pub fn button_for_key(code: i32) -> Option<GamepadButton> {
    use GamepadButton::*;
    Some(match code {
        19 => DPadUp,
        20 => DPadDown,
        21 => DPadLeft,
        22 => DPadRight,
        23 | 66 | 96 => South, // DPAD_CENTER, ENTER, BUTTON_A
        4 | 97 => East,        // BACK, BUTTON_B
        99 => West,            // BUTTON_X
        100 => North,          // BUTTON_Y
        102 => LeftTrigger,    // L1
        103 => RightTrigger,   // R1
        104 => LeftTrigger2,   // L2 (digital-only pads)
        105 => RightTrigger2,  // R2
        106 => LeftThumb,
        107 => RightThumb,
        108 | 82 | 85 => Start, // BUTTON_START, MENU, MEDIA_PLAY_PAUSE
        109 => Select,
        110 => Mode,
        _ => return None,
    })
}

#[derive(Debug, PartialEq)]
pub enum Change {
    Button(GamepadButton, f32),
    Axis(GamepadAxis, f32),
}

/// Android `MotionEvent.AXIS_*` → Bevy changes. Android's stick Y is down-positive, Bevy's up-positive; the hat
/// becomes D-pad buttons and the analog triggers become `*Trigger2` buttons with their pressure.
pub fn axis_changes(code: i32, v: f32) -> Vec<Change> {
    use Change::*;
    use GamepadButton::*;
    let on = |b: bool| if b { 1.0 } else { 0.0 };
    match code {
        0 => vec![Axis(GamepadAxis::LeftStickX, v)],
        1 => vec![Axis(GamepadAxis::LeftStickY, -v)],
        11 => vec![Axis(GamepadAxis::RightStickX, v)],
        14 => vec![Axis(GamepadAxis::RightStickY, -v)],
        15 => vec![Button(DPadLeft, on(v < -0.5)), Button(DPadRight, on(v > 0.5))],
        16 => vec![Button(DPadUp, on(v < -0.5)), Button(DPadDown, on(v > 0.5))],
        17 | 23 => vec![Button(LeftTrigger2, v)],  // LTRIGGER, BRAKE
        18 | 22 => vec![Button(RightTrigger2, v)], // RTRIGGER, GAS
        _ => vec![],
    }
}

/// Filled by the JNI callback on Android's UI thread, drained once per frame.
static QUEUE: Mutex<Vec<(i32, i32, i32, f32)>> = Mutex::new(Vec::new());
/// Rumble for Java to pick up: (Android id, strong, weak, ms).
static RUMBLE: Mutex<Vec<(i32, f32, f32, u32)>> = Mutex::new(Vec::new());

#[derive(Resource, Default)]
struct Pads {
    by_id: HashMap<i32, Entity>,
    by_entity: HashMap<Entity, i32>,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Pads>().add_systems(PreUpdate, pump.before(bevy::input::InputSystems)).add_systems(PostUpdate, rumble);
}

fn pump(
    mut commands: Commands,
    mut pads: ResMut<Pads>,
    mut raw: MessageWriter<RawGamepadEvent>,
    mut connections: MessageWriter<GamepadConnectionEvent>,
    mut buttons: MessageWriter<RawGamepadButtonChangedEvent>,
    mut axes: MessageWriter<RawGamepadAxisChangedEvent>,
) {
    let events = std::mem::take(&mut *QUEUE.lock().unwrap());
    for (id, kind, code, value) in events {
        if kind == CONNECT {
            let pad = match pads.by_id.get(&id) {
                Some(&e) => e,
                None => {
                    let e = commands.spawn_empty().id();
                    pads.by_id.insert(id, e);
                    pads.by_entity.insert(e, id);
                    e
                }
            };
            let remote = code & REMOTE_FLAG != 0;
            if remote {
                commands.entity(pad).insert(TvRemote);
            }
            let (vendor, product) = ((code >> 16) as u16 & 0x7FFF, code as u16);
            let name = if remote { "TV remote" } else { "Android gamepad" }.to_string();
            let event =
                GamepadConnectionEvent::new(pad, GamepadConnection::Connected { name, vendor_id: Some(vendor), product_id: Some(product) });
            raw.write(event.clone().into());
            connections.write(event);
            continue;
        }
        let Some(&pad) = pads.by_id.get(&id) else { continue };
        match kind {
            DISCONNECT => {
                let event = GamepadConnectionEvent::new(pad, GamepadConnection::Disconnected);
                raw.write(event.clone().into());
                connections.write(event);
            }
            KEY => {
                if let Some(b) = button_for_key(code) {
                    let e = RawGamepadButtonChangedEvent::new(pad, b, value);
                    raw.write(e.into());
                    buttons.write(e);
                }
            }
            AXIS => {
                for change in axis_changes(code, value) {
                    match change {
                        Change::Button(b, v) => {
                            let e = RawGamepadButtonChangedEvent::new(pad, b, v);
                            raw.write(e.into());
                            buttons.write(e);
                        }
                        Change::Axis(a, v) => {
                            let e = RawGamepadAxisChangedEvent::new(pad, a, v);
                            raw.write(e.into());
                            axes.write(e);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

fn rumble(pads: Res<Pads>, mut requests: MessageReader<GamepadRumbleRequest>) {
    for r in requests.read() {
        if let GamepadRumbleRequest::Add { gamepad, duration, intensity } = r
            && let Some(&id) = pads.by_entity.get(gamepad)
        {
            RUMBLE.lock().unwrap().push((id, intensity.strong_motor, intensity.weak_motor, duration.as_millis() as u32));
        }
    }
}

/// `PadHub.nativePad(int id, int kind, int code, float value)`, called on Android's UI thread.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_lunatiq_battletanks_PadHub_nativePad(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
    id: i32,
    kind: i32,
    code: i32,
    value: f32,
) {
    QUEUE.lock().unwrap().push((id, kind, code, value));
}

/// `PadHub.nativePollRumble()`: one pending rumble packed as id(32) | strong(8) | weak(8) | ms(16), or 0.
#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_lunatiq_battletanks_PadHub_nativePollRumble(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
) -> i64 {
    RUMBLE.lock().unwrap().pop().map_or(0, |(id, strong, weak, ms)| pack_rumble(id, strong, weak, ms))
}

pub fn pack_rumble(id: i32, strong: f32, weak: f32, ms: u32) -> i64 {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as i64;
    ((id as u32 as i64) << 32) | (byte(strong) << 24) | (byte(weak) << 16) | ms.min(0xFFFF) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_and_pad_buttons_map_to_the_same_actions() {
        assert_eq!(button_for_key(23), Some(GamepadButton::South)); // remote OK
        assert_eq!(button_for_key(96), Some(GamepadButton::South)); // pad A
        assert_eq!(button_for_key(4), Some(GamepadButton::East)); // remote Back
        assert_eq!(button_for_key(108), Some(GamepadButton::Start));
        assert_eq!(button_for_key(24), None); // volume up stays with the TV
    }

    #[test]
    fn stick_y_flips_and_hat_becomes_dpad() {
        assert_eq!(axis_changes(1, 1.0), vec![Change::Axis(GamepadAxis::LeftStickY, -1.0)]);
        assert_eq!(axis_changes(16, -1.0), vec![Change::Button(GamepadButton::DPadUp, 1.0), Change::Button(GamepadButton::DPadDown, 0.0)]);
        assert_eq!(
            axis_changes(15, 0.0),
            vec![Change::Button(GamepadButton::DPadLeft, 0.0), Change::Button(GamepadButton::DPadRight, 0.0)]
        );
        assert_eq!(axis_changes(22, 0.8), vec![Change::Button(GamepadButton::RightTrigger2, 0.8)]);
    }

    #[test]
    fn rumble_packs_into_one_long() {
        let p = pack_rumble(7, 1.0, 0.5, 450);
        assert_eq!(p >> 32, 7);
        assert_eq!((p >> 24) & 0xFF, 255);
        assert_eq!((p >> 16) & 0xFF, 128);
        assert_eq!(p & 0xFFFF, 450);
    }
}
