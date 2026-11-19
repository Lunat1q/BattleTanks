//! Devices -> per-frame `Intent`. Controllers first; two keyboard layouts so two people can share
//! one keyboard. Everything downstream reads `DeviceIntents` / `Intents`, never raw input.

use crate::game::{Rumble, Session};
use crate::geom::Dir;
use crate::settings::Settings;
use bevy::input::gamepad::{Gamepad, GamepadButton, GamepadRumbleIntensity, GamepadRumbleRequest};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Device {
    /// WASD + Space/F, Esc pauses.
    KeysLeft,
    /// Arrows + Enter/Right Ctrl/Numpad 0, Esc pauses.
    KeysRight,
    Pad(Entity),
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Intent {
    /// Held movement direction.
    pub dir: Option<Dir>,
    pub fire: bool,
    pub fire_pressed: bool,
    /// A / Space / Enter just pressed (menus, join).
    pub confirm: bool,
    /// B / Backspace just pressed.
    pub back: bool,
    /// Start / Esc just pressed.
    pub start: bool,
    /// Toggles pause like Start: Back on a TV remote, which has no Start. Only the stage reads it, so Back keeps
    /// meaning "back" in other menus; the pause menu ignores a Back that set this, so it isn't handled twice.
    pub pause: bool,
    /// Direction just pressed (menu navigation).
    pub nav: Option<Dir>,
}

/// Intent of every connected device this frame (title screen joins from these).
#[derive(Resource, Default)]
pub struct DeviceIntents(pub Vec<(Device, Intent)>);

impl DeviceIntents {
    pub fn get(&self, d: Device) -> Intent {
        self.0.iter().find(|(dev, _)| *dev == d).map(|(_, i)| *i).unwrap_or_default()
    }
    pub fn connected(&self, d: Device) -> bool {
        self.0.iter().any(|(dev, _)| *dev == d)
    }
}

pub fn merge(a: Intent, b: Intent) -> Intent {
    Intent {
        dir: a.dir.or(b.dir),
        fire: a.fire || b.fire,
        fire_pressed: a.fire_pressed || b.fire_pressed,
        confirm: a.confirm || b.confirm,
        back: a.back || b.back,
        start: a.start || b.start,
        pause: a.pause || b.pause,
        nav: a.nav.or(b.nav),
    }
}

const STICK_DEAD_ZONE: f32 = 0.5;

/// A gamepad that is really a TV remote (tagged by a host that feeds its own gamepads): it has no Start, so Back
/// doubles as pause.
#[derive(Component)]
pub struct TvRemote;

pub fn plugin(app: &mut App) {
    app.init_resource::<DeviceIntents>().add_systems(PreUpdate, gather.after(bevy::input::InputSystems)).add_systems(PostUpdate, rumble);
}

fn keys_dir(keys: &ButtonInput<KeyCode>, map: [KeyCode; 4], pressed: bool) -> Option<Dir> {
    let check = |k| if pressed { keys.pressed(k) } else { keys.just_pressed(k) };
    // Latest-wins would be nicer; ponytail: fixed priority is fine for 4-way tanks.
    Dir::ALL.into_iter().zip(map).find(|&(_, k)| check(k)).map(|(d, _)| d)
}

fn keyboard(keys: &ButtonInput<KeyCode>, moves: [KeyCode; 4], fire: &[KeyCode], back: KeyCode) -> Intent {
    Intent {
        dir: keys_dir(keys, moves, true),
        fire: keys.any_pressed(fire.iter().copied()),
        fire_pressed: keys.any_just_pressed(fire.iter().copied()),
        confirm: keys.any_just_pressed(fire.iter().copied()),
        back: keys.just_pressed(back),
        start: keys.just_pressed(KeyCode::Escape),
        pause: false,
        nav: keys_dir(keys, moves, false),
    }
}

fn pad_intent(pad: &Gamepad, remote: bool, last_stick: &mut Option<Dir>) -> Intent {
    use GamepadButton::*;
    let dpad = [DPadUp, DPadDown, DPadLeft, DPadRight];
    let dpad_dir = Dir::ALL.into_iter().zip(dpad).find(|&(_, b)| pad.pressed(b)).map(|(d, _)| d);
    let stick = Dir::from_vec(pad.left_stick(), STICK_DEAD_ZONE);
    let stick_edge = if stick != *last_stick { stick } else { None };
    *last_stick = stick;
    let fire = [South, West, RightTrigger, RightTrigger2];
    Intent {
        dir: dpad_dir.or(stick),
        fire: fire.iter().any(|&b| pad.pressed(b)),
        fire_pressed: fire.iter().any(|&b| pad.just_pressed(b)),
        confirm: pad.just_pressed(South),
        back: pad.just_pressed(East),
        start: pad.just_pressed(Start),
        pause: remote && pad.just_pressed(East),
        nav: Dir::ALL.into_iter().zip(dpad).find(|&(_, b)| pad.just_pressed(b)).map(|(d, _)| d).or(stick_edge),
    }
}

fn gather(
    keys: Res<ButtonInput<KeyCode>>,
    pads: Query<(Entity, &Gamepad, Has<TvRemote>)>,
    mut sticks: Local<HashMap<Entity, Option<Dir>>>,
    mut out: ResMut<DeviceIntents>,
) {
    use KeyCode::*;
    out.0.clear();
    out.0.push((Device::KeysLeft, keyboard(&keys, [KeyW, KeyS, KeyA, KeyD], &[Space, KeyF], Backspace)));
    out.0.push((
        Device::KeysRight,
        keyboard(&keys, [ArrowUp, ArrowDown, ArrowLeft, ArrowRight], &[Enter, NumpadEnter, ControlRight, Numpad0], Backspace),
    ));
    for (e, pad, remote) in &pads {
        out.0.push((Device::Pad(e), pad_intent(pad, remote, sticks.entry(e).or_default())));
    }
}

/// Gameplay `Rumble` -> the slot's controller (keyboards and vibration-off just drop it). Delayed pulses wait in
/// `later` (real time, so they play out over a pause or a screen change).
fn rumble(
    settings: Res<Settings>,
    session: Res<Session>,
    time: Res<Time<Real>>,
    mut later: Local<Vec<Rumble>>,
    mut events: MessageReader<Rumble>,
    mut out: MessageWriter<GamepadRumbleRequest>,
) {
    let dt = time.delta_secs();
    later.extend(events.read().copied());
    let mut due = Vec::new();
    later.retain_mut(|r| {
        r.delay -= dt;
        let wait = r.delay > 0.0;
        if !wait {
            due.push(*r);
        }
        wait
    });
    for r in &due {
        let Some(Device::Pad(gamepad)) = session.slots.get(r.slot).and_then(|s| s.as_ref()).map(|s| s.device) else { continue };
        if settings.vibration {
            out.write(GamepadRumbleRequest::Add {
                gamepad,
                duration: std::time::Duration::from_secs_f32(r.secs),
                intensity: GamepadRumbleIntensity { strong_motor: r.strong, weak_motor: r.weak },
            });
        }
    }
}
