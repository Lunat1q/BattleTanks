//! Title: players join by pressing A (or Space / Enter), then pick START. B leaves a slot.

use super::*;
use crate::game::{Mode, Screen, Session, Sfx, Slot};
use crate::geom::Dir;
use crate::gfx::Gfx;
use crate::input::{Device, DeviceIntents, Intent};
use crate::levels;
use crate::net::Net;

#[derive(Resource)]
pub(crate) struct TitleMenu {
    pub(crate) sel: usize,
    pub(crate) stage: usize,
}

#[derive(Component)]
struct SlotLine(usize);

/// Linked: the measured round trip and the input delay it picked.
#[derive(Component)]
struct PingLine;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Item {
    Start,
    Stage,
    City,
    Lan,
    HiScores,
    Achievements,
    Settings,
    Exit,
    Leave,
}

/// The menu. Linked, only what both machines can do together: no online boards, and no settings (the host's apply).
fn items(linked: bool, platform: &crate::Platform) -> Vec<Item> {
    use Item::*;
    if linked {
        return vec![Start, Stage, City, Leave];
    }
    [Start, Stage, City]
        .into_iter()
        .chain(platform.lan.is_some().then_some(Lan))
        .chain([HiScores])
        .chain(platform.achieve.is_some().then_some(Achievements))
        .chain([Settings, Exit])
        .collect()
}

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
    session.log.clear();
}

fn spawn(mut commands: Commands, session: Res<Session>, gfx: Res<Gfx>, net: Res<Net>, platform: Res<crate::Platform>) {
    // Version in the corner, so a TV screenshot says which build it was.
    commands.spawn((
        text(concat!("v", env!("CARGO_PKG_VERSION")), 2.4, DIM),
        Node { position_type: PositionType::Absolute, bottom: vh(1.5), right: vh(2), ..default() },
        DespawnOnExit(Screen::Title),
    ));
    let n = items(net.linked(), &platform).len();
    // Seven items fill the screen; a longer menu (LAN + ACHIEVEMENTS) packs tighter so the bottom lines stay on it.
    let (item_vh, gap) = if n > 7 { (4.2, vh(1.5)) } else { (5.0, vh(4)) };
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
        root.spawn(Node { height: gap, ..default() });
        for i in 0..n {
            root.spawn((text("", item_vh, INK), MenuLine(i)));
        }
        root.spawn(Node { height: gap, ..default() });
        root.spawn(Node { column_gap: vw(8), ..default() }).with_children(|row| {
            for i in 0..2 {
                row.spawn((text("", 3.6, if i == 0 { GOLD } else { GREEN }), SlotLine(i)));
            }
        });
        root.spawn(Node { height: gap, ..default() });
        root.spawn(text(format!("HI SCORE  {}", session.hi_score), 3.0, DIM));
        if net.linked() {
            root.spawn((text("", 2.6, DIM), PingLine));
        }
        if !net.linked() && !net.note.is_empty() {
            root.spawn(text(format!("LAN: {}", net.note), 2.6, RED));
        }
        root.spawn(text("Controller: stick/D-pad move  A fire  Start pause  |  Keys: P1 WASD+Space  P2 Arrows+Enter  Esc pause", 2.2, DIM));
    });
}

fn device_name(d: Device, net: &Net) -> String {
    match d {
        Device::KeysLeft => "KEYS WASD".into(),
        Device::KeysRight => "KEYS ARROWS".into(),
        Device::Pad(_) => "CONTROLLER".into(),
        Device::Net(i) => net.names[i as usize].to_uppercase(),
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
    mut net: ResMut<Net>,
) {
    let items = items(net.linked(), &platform);
    let n = items.len();
    menu.sel = menu.sel.min(n - 1);
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
    if platform.back_exits_title && !net.linked() && joined.is_empty() && devices.0.iter().any(|(_, i)| i.back) {
        exit.write(AppExit::Success);
    }
    match nav.nav {
        Some(Dir::Up) => menu.sel = (menu.sel + n - 1) % n,
        Some(Dir::Down) => menu.sel = (menu.sel + 1) % n,
        Some(Dir::Left) if menu.sel == 1 => menu.stage = if menu.stage <= 1 { levels::stage_count() } else { menu.stage - 1 },
        Some(Dir::Right) if menu.sel == 1 => menu.stage = menu.stage % levels::stage_count() + 1,
        _ => {}
    }
    if nav.nav.is_some() {
        sfx.write(Sfx::MenuMove);
    }
    if nav.confirm || nav.start {
        match items[menu.sel] {
            Item::Start | Item::Stage if session.players() > 0 => {
                session.mode = Mode::Classic;
                session.stage = menu.stage;
                sfx.write(Sfx::MenuPick);
                next.set(Screen::Stage);
            }
            Item::City if session.players() > 0 => {
                session.mode = Mode::City;
                sfx.write(Sfx::MenuPick);
                next.set(Screen::CityMap);
            }
            Item::Lan => {
                sfx.write(Sfx::MenuPick);
                next.set(Screen::Lan);
            }
            Item::HiScores => {
                sfx.write(Sfx::MenuPick);
                next.set(Screen::HiScores);
            }
            Item::Achievements => {
                sfx.write(Sfx::MenuPick);
                next.set(Screen::Achievements);
            }
            Item::Settings => next.set(Screen::Settings),
            Item::Exit => {
                exit.write(AppExit::Success);
            }
            Item::Leave => {
                sfx.write(Sfx::MenuPick);
                net.leave = true;
            }
            _ => {}
        }
    }
}

fn refresh(
    menu: Res<TitleMenu>,
    session: Res<Session>,
    net: Res<Net>,
    platform: Res<crate::Platform>,
    mut lines: Query<(&mut Text, Option<&MenuLine>, Option<&SlotLine>, Has<PingLine>)>,
) {
    let items = items(net.linked(), &platform);
    for (mut t, line, slot, ping) in &mut lines {
        if ping {
            let s = match net.ping {
                Some((rtt, delay)) => format!("LAN  round trip {rtt} ms  -  input delay {delay} ({} ms)", (delay * 50).div_ceil(3)),
                None => "LAN  measuring the connection...".into(),
            };
            set_text(&mut t, s);
        }
        if let Some(MenuLine(i)) = line {
            let label = match items.get(*i) {
                Some(Item::Start) if session.players() == 0 => "PRESS A TO JOIN".to_string(),
                Some(Item::Start) => "START".to_string(),
                Some(Item::Stage) => format!("STAGE  < {} >", menu.stage),
                Some(Item::City) => "CITY RUN".to_string(),
                Some(Item::Lan) => "LAN CO-OP".to_string(),
                Some(Item::HiScores) => "HI SCORES".to_string(),
                Some(Item::Achievements) => "ACHIEVEMENTS".to_string(),
                Some(Item::Settings) => "SETTINGS".to_string(),
                Some(Item::Exit) => "EXIT".to_string(),
                Some(Item::Leave) => "LEAVE LAN".to_string(),
                None => String::new(),
            };
            set_text(&mut t, menu_label(menu.sel == *i, &label));
        }
        if let Some(SlotLine(i)) = slot {
            let s = match &session.slots[*i] {
                Some(s) => format!("{}P  {}  READY", i + 1, device_name(s.device, &net)),
                None => format!("{}P  press A to join", i + 1),
            };
            set_text(&mut t, s);
        }
    }
}
