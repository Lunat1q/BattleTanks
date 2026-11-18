//! Title: players join by pressing A (or Space / Enter), then pick START. B leaves a slot.

use super::*;
use crate::game::{Mode, Screen, Session, Sfx, Slot};
use crate::geom::Dir;
use crate::gfx::Gfx;
use crate::input::{Device, DeviceIntents, Intent};
use crate::levels;

#[derive(Resource)]
struct TitleMenu {
    sel: usize,
    stage: usize,
}

#[derive(Component)]
struct SlotLine(usize);

const ITEMS: usize = 5; // START, STAGE, CITY RUN, SETTINGS, EXIT

pub fn plugin(app: &mut App) {
    app.insert_resource(TitleMenu { sel: 0, stage: 1 })
        .add_systems(OnEnter(Screen::Title), (reset_session, spawn).chain())
        .add_systems(Update, (input, refresh).chain().run_if(in_state(Screen::Title)));
}

/// Back on the title: keep who joined (if still connected), reset their game stats.
fn reset_session(mut session: ResMut<Session>, devices: Res<DeviceIntents>) {
    for slot in session.slots.iter_mut() {
        *slot = slot.take().filter(|s| devices.connected(s.device)).map(|s| Slot::new(s.device));
    }
}

fn spawn(mut commands: Commands, session: Res<Session>, gfx: Res<Gfx>) {
    // Version in the corner, so a TV screenshot says which build it was.
    commands.spawn((
        text(concat!("v", env!("CARGO_PKG_VERSION")), 2.4, DIM),
        Node { position_type: PositionType::Absolute, bottom: vh(1.5), right: vh(2), ..default() },
        DespawnOnExit(Screen::Title),
    ));
    commands.spawn((screen_root(), DespawnOnExit(Screen::Title))).with_children(|root| {
        root.spawn(Node { align_items: AlignItems::Center, column_gap: vh(4), ..default() }).with_children(|row| {
            let tank = |slot: usize, color| {
                (
                    ImageNode::new(gfx.players[3][0].clone()).with_color(color),
                    Node { width: vh(14), height: vh(14), ..default() },
                    UiTransform::from_rotation(Rot2::degrees(if slot == 0 { 90.0 } else { -90.0 })),
                )
            };
            row.spawn(tank(0, GOLD));
            row.spawn(text("BATTLE TANKS", 13.0, GOLD));
            row.spawn(tank(1, GREEN));
        });
        root.spawn(text("two-player tank defence", 3.2, DIM));
        root.spawn(Node { height: vh(4), ..default() });
        for i in 0..ITEMS {
            root.spawn((text("", 5.0, INK), MenuLine(i)));
        }
        root.spawn(Node { height: vh(4), ..default() });
        root.spawn(Node { column_gap: vw(8), ..default() }).with_children(|row| {
            for i in 0..2 {
                row.spawn((text("", 3.6, if i == 0 { GOLD } else { GREEN }), SlotLine(i)));
            }
        });
        root.spawn(Node { height: vh(4), ..default() });
        root.spawn(text(format!("HI SCORE  {}", session.hi_score), 3.0, DIM));
        root.spawn(text("Controller: stick/D-pad move  A fire  Start pause  |  Keys: P1 WASD+Space  P2 Arrows+Enter  Esc pause", 2.2, DIM));
    });
}

fn device_name(d: Device) -> String {
    match d {
        Device::KeysLeft => "KEYS WASD".into(),
        Device::KeysRight => "KEYS ARROWS".into(),
        Device::Pad(_) => "CONTROLLER".into(),
    }
}

fn input(
    devices: Res<DeviceIntents>,
    mut session: ResMut<Session>,
    mut menu: ResMut<TitleMenu>,
    mut next: ResMut<NextState<Screen>>,
    mut exit: MessageWriter<AppExit>,
    mut sfx: MessageWriter<Sfx>,
    platform: Res<crate::Platform>,
) {
    let joined: Vec<Device> = session.slots.iter().flatten().map(|s| s.device).collect();
    let mut nav = Intent::default();
    for &(dev, intent) in &devices.0 {
        if joined.contains(&dev) {
            if intent.back {
                session.slots.iter_mut().for_each(|s| {
                    if s.as_ref().is_some_and(|s| s.device == dev) {
                        *s = None;
                    }
                });
                sfx.write(Sfx::MenuMove);
                continue;
            }
            nav = crate::input::merge(nav, intent);
        } else if intent.confirm {
            // Not Start: Esc is shared by both keyboard layouts and would join both.
            if let Some(free) = session.slots.iter_mut().find(|s| s.is_none()) {
                *free = Some(Slot::new(dev));
                sfx.write(Sfx::MenuPick);
            }
        } else {
            // Unjoined devices can still move the cursor (e.g. to reach EXIT).
            nav.nav = nav.nav.or(intent.nav);
        }
    }
    // TV launcher convention: Back on the app's first screen goes home. Only with nobody joined, so a joined
    // player's Back still just leaves their slot.
    if platform.back_exits_title && joined.is_empty() && devices.0.iter().any(|(_, i)| i.back) {
        exit.write(AppExit::Success);
    }
    match nav.nav {
        Some(Dir::Up) => menu.sel = (menu.sel + ITEMS - 1) % ITEMS,
        Some(Dir::Down) => menu.sel = (menu.sel + 1) % ITEMS,
        Some(Dir::Left) if menu.sel == 1 => menu.stage = if menu.stage <= 1 { levels::stage_count() } else { menu.stage - 1 },
        Some(Dir::Right) if menu.sel == 1 => menu.stage = menu.stage % levels::stage_count() + 1,
        _ => {}
    }
    if nav.nav.is_some() {
        sfx.write(Sfx::MenuMove);
    }
    if nav.confirm || nav.start {
        match menu.sel {
            0 | 1 if session.players() > 0 => {
                session.mode = Mode::Classic;
                session.stage = menu.stage;
                sfx.write(Sfx::MenuPick);
                next.set(Screen::Stage);
            }
            2 if session.players() > 0 => {
                session.mode = Mode::City;
                sfx.write(Sfx::MenuPick);
                next.set(Screen::CityMap);
            }
            3 => next.set(Screen::Settings),
            4 => {
                exit.write(AppExit::Success);
            }
            _ => {}
        }
    }
}

fn refresh(menu: Res<TitleMenu>, session: Res<Session>, mut lines: Query<(&mut Text, Option<&MenuLine>, Option<&SlotLine>)>) {
    for (mut t, line, slot) in &mut lines {
        if let Some(MenuLine(i)) = line {
            let label = match i {
                0 if session.players() == 0 => "PRESS A TO JOIN".to_string(),
                0 => "START".to_string(),
                1 => format!("STAGE  < {} >", menu.stage),
                2 => "CITY RUN".to_string(),
                3 => "SETTINGS".to_string(),
                _ => "EXIT".to_string(),
            };
            set_text(&mut t, menu_label(menu.sel == *i, &label));
        }
        if let Some(SlotLine(i)) = slot {
            let s = match &session.slots[*i] {
                Some(s) => format!("{}P  {}  READY", i + 1, device_name(s.device)),
                None => format!("{}P  press A to join", i + 1),
            };
            set_text(&mut t, s);
        }
    }
}
