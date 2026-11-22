//! Title: players join by pressing A (or Space / Enter) on a card, player 1 left of the menu, player 2 right; B leaves
//! a slot.
//! CLASSIC goes to the stage tiles, CITY RUN to the district map. `COMBO` opens the secret bunker.

use super::*;
use crate::game::{Mode, Screen, Session, Sfx, Slot};
use crate::geom::Dir;
use crate::gfx::Gfx;
use crate::input::{Device, DeviceIntents, Intent};
use crate::net::Net;

#[derive(Resource, Default)]
pub(crate) struct TitleMenu {
    pub(crate) sel: usize,
    /// The last presses, for `COMBO`.
    combo: Vec<Key>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Key {
    Go(Dir),
    A,
}

/// Road safety for tanks: look left, right, left again, cross (up, up), honk (A). D-pad and A only, so a TV remote
/// and the touch controls can do it too; no B, which leaves a slot or (TV) quits from here. The moves are harmless
/// on the title (left/right do nothing, up moves the cursor) and the A that finishes it doesn't join or pick.
const COMBO: [Key; 6] = [Key::Go(Dir::Left), Key::Go(Dir::Right), Key::Go(Dir::Left), Key::Go(Dir::Up), Key::Go(Dir::Up), Key::A];

/// Remembers `k`; true when the last presses spell `COMBO`.
fn combo(keys: &mut Vec<Key>, k: Key) -> bool {
    keys.push(k);
    if keys.len() > COMBO.len() {
        keys.remove(0);
    }
    keys[..] == COMBO
}

/// Player card `n`: its frame and its two text lines.
#[derive(Component)]
struct Card(usize);
#[derive(Component)]
struct CardLine(usize, usize);

/// Player `slot`'s tank beside the logo: drives in from its side of the screen when the player joins (`at` 0 = off
/// screen, 1 = parked), backs out when they leave.
#[derive(Component)]
struct TitleTank {
    slot: usize,
    at: f32,
    tread: f32,
}

/// Seconds for the drive in.
const DRIVE: f32 = 1.8;

/// Linked: the measured round trip and the input delay it picked.
#[derive(Component)]
struct PingLine;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Item {
    Classic,
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
        return vec![Classic, City, Leave];
    }
    [Classic, City]
        .into_iter()
        .chain(platform.lan.is_some().then_some(Lan))
        .chain([HiScores])
        .chain(platform.achieve.is_some().then_some(Achievements))
        .chain([Settings, Exit])
        .collect()
}

const SLOT_COLORS: [Color; 2] = [GOLD, GREEN];

pub fn plugin(app: &mut App) {
    app.init_resource::<TitleMenu>()
        .add_systems(OnEnter(Screen::Title), (reset_session, spawn).chain())
        .add_systems(Update, (input, refresh, drive).chain().run_if(in_state(Screen::Title)));
}

/// Back on the title: keep who joined (if still connected), reset their game stats.
fn reset_session(mut session: ResMut<Session>, devices: Res<DeviceIntents>) {
    for slot in session.slots.iter_mut() {
        *slot = slot.take().filter(|s| devices.connected(s.device)).map(|s| Slot::new(s.device));
    }
    session.log.clear();
}

/// The controls that this host has, one line each.
fn hints(platform: &crate::Platform) -> Vec<&'static str> {
    let pad = "stick / D-pad move   A fire / pick   B back   Start pause";
    let keys = "keys: 1P WASD + Space   2P arrows + Enter   Backspace back   Esc pause";
    std::iter::once(pad).chain(platform.keyboard.then_some(keys)).collect()
}

fn spawn(mut commands: Commands, session: Res<Session>, gfx: Res<Gfx>, net: Res<Net>, platform: Res<crate::Platform>) {
    // Version in the corner, so a TV screenshot says which build it was.
    commands.spawn((
        text(concat!("v", env!("CARGO_PKG_VERSION")), 2.4, DIM),
        Node { position_type: PositionType::Absolute, bottom: vh(1.5), right: vh(2), ..default() },
        DespawnOnExit(Screen::Title),
    ));
    let n = items(net.linked(), &platform).len();
    // The frame is 30 x 22 chunky pixels, so one pixel is 1 vh.
    let card = |slot: usize| {
        (
            Node {
                width: vh(30),
                height: vh(22),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                row_gap: vh(1.4),
                ..default()
            },
            ImageNode::new(gfx.card_frame.clone()).with_color(SLOT_COLORS[slot]),
            Card(slot),
        )
    };
    commands.spawn((screen_root(), DespawnOnExit(Screen::Title))).with_children(|root| {
        root.spawn(Node { align_items: AlignItems::Center, column_gap: vh(4), ..default() }).with_children(|row| {
            // Players already in (back from a game) are parked; the others wait off screen.
            let tank = |slot: usize| {
                let at = if session.slots[slot].is_some() { 1.0 } else { 0.0 };
                (
                    ImageNode::new(gfx.players[3][0].clone()).with_color(SLOT_COLORS[slot]),
                    Node { width: vh(14), height: vh(14), ..default() },
                    UiTransform {
                        translation: Val2::new(Val::Vw(tank_offset(slot, at)), Val::ZERO),
                        ..UiTransform::from_rotation(Rot2::degrees(if slot == 0 { 90.0 } else { -90.0 }))
                    },
                    TitleTank { slot, at, tread: 0.0 },
                )
            };
            row.spawn(tank(0));
            row.spawn((ImageNode::new(gfx.title_logo.clone()), Node { height: vh(12), ..default() }));
            row.spawn(tank(1));
        });
        root.spawn(Node { height: vh(3), ..default() });
        root.spawn(Node { align_items: AlignItems::Center, column_gap: vh(10), ..default() }).with_children(|row| {
            for (slot, &color) in SLOT_COLORS.iter().enumerate() {
                if slot == 1 {
                    row.spawn(Node {
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: vh(1.8),
                        // Fixed width: the selection marks ("> ... <") must not shift the cards.
                        width: vh(54),
                        ..default()
                    })
                    .with_children(|menu| {
                        for i in 0..n {
                            menu.spawn((text("", 4.8, INK), MenuLine(i)));
                        }
                    });
                }
                row.spawn(card(slot)).with_children(|c| {
                    c.spawn(text(format!("PLAYER {}", slot + 1), 4.4, color));
                    c.spawn((text("", 2.8, INK), CardLine(slot, 0)));
                    c.spawn((text("", 2.4, DIM), CardLine(slot, 1)));
                });
            }
        });
        root.spawn(Node { height: vh(3), ..default() });
        root.spawn(text(format!("HI SCORE  {}", session.hi_score), 3.0, DIM));
        if net.linked() {
            root.spawn((text("", 2.6, DIM), PingLine));
        }
        if !net.linked() && !net.note.is_empty() {
            root.spawn(text(format!("LAN: {}", net.note), 2.6, RED));
        }
        for hint in hints(&platform) {
            root.spawn(text(hint, 2.2, DIM));
        }
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
    // Offline only: a linked pair changes screens together.
    if !net.linked() {
        let presses = devices.0.iter().flat_map(|(_, i)| i.nav.map(Key::Go).into_iter().chain(i.confirm.then_some(Key::A)));
        let presses: Vec<Key> = presses.collect();
        if presses.into_iter().any(|k| combo(&mut menu.combo, k)) {
            menu.combo.clear();
            sfx.write(Sfx::ExtraLife);
            next.set(Screen::Bunker);
            return;
        }
    }
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
            // Devices that haven't joined can still move the cursor (e.g. to reach EXIT).
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
        _ => {}
    }
    if matches!(nav.nav, Some(Dir::Up | Dir::Down)) {
        sfx.write(Sfx::MenuMove);
    }
    if nav.confirm || nav.start {
        let to = match items[menu.sel] {
            Item::Classic if session.players() > 0 => {
                session.mode = Mode::Classic;
                Some(Screen::StageSelect)
            }
            Item::City if session.players() > 0 => {
                session.mode = Mode::City;
                Some(Screen::CityMap)
            }
            Item::Lan => Some(Screen::Lan),
            Item::HiScores => Some(Screen::HiScores),
            Item::Achievements => Some(Screen::Achievements),
            Item::Settings => Some(Screen::Settings),
            Item::Exit => {
                exit.write(AppExit::Success);
                None
            }
            Item::Leave => {
                sfx.write(Sfx::MenuPick);
                net.leave = true;
                None
            }
            _ => None,
        };
        if let Some(to) = to {
            if to != Screen::Settings {
                sfx.write(Sfx::MenuPick);
            }
            next.set(to);
        }
    }
}

/// What an empty card says: a controller that is on but hasn't joined, or just how to join.
fn empty_card(free_pad: bool, keyboard: bool) -> (&'static str, &'static str) {
    match (free_pad, keyboard) {
        (true, _) => ("CONTROLLER FOUND", "PRESS A TO JOIN"),
        (false, true) => ("PRESS A TO JOIN", "or Space / Enter"),
        (false, false) => ("PRESS A TO JOIN", ""),
    }
}

fn refresh(
    time: Res<Time>,
    menu: Res<TitleMenu>,
    session: Res<Session>,
    net: Res<Net>,
    devices: Res<DeviceIntents>,
    platform: Res<crate::Platform>,
    mut lines: Query<(&mut Text, &mut TextColor, Option<&MenuLine>, Option<&CardLine>, Has<PingLine>)>,
    mut cards: Query<(&Card, &mut ImageNode)>,
) {
    let items = items(net.linked(), &platform);
    let blink = 0.7 + 0.3 * (time.elapsed_secs() * 4.0).sin();
    let joined: Vec<Device> = session.slots.iter().flatten().map(|s| s.device).collect();
    // Pads that are on but in no slot, handed to the empty cards in order (offline only: linked, pads are this
    // machine's alone and the cards show both machines).
    let mut free = devices.0.iter().filter(|(d, _)| matches!(d, Device::Pad(_)) && !joined.contains(d)).filter(|_| !net.linked());
    let card_text: Vec<(String, String)> = (0..2)
        .map(|i| match &session.slots[i] {
            Some(s) => (device_name(s.device, &net), "READY   B leave".into()),
            None => {
                let (a, b) = empty_card(free.next().is_some(), platform.keyboard && !net.linked());
                (a.into(), b.into())
            }
        })
        .collect();
    for (mut t, mut color, line, card, ping) in &mut lines {
        if ping {
            let s = match net.ping {
                Some((rtt, delay)) => format!("LAN  round trip {rtt} ms  -  input delay {delay} ({} ms)", (delay * 50).div_ceil(3)),
                None => "LAN  measuring the connection...".into(),
            };
            set_text(&mut t, s);
        }
        if let Some(MenuLine(i)) = line {
            let label = match items.get(*i) {
                Some(Item::Classic) => "CLASSIC",
                Some(Item::City) => "CITY RUN",
                Some(Item::Lan) => "LAN CO-OP",
                Some(Item::HiScores) => "HI SCORES",
                Some(Item::Achievements) => "ACHIEVEMENTS",
                Some(Item::Settings) => "SETTINGS",
                Some(Item::Exit) => "EXIT",
                Some(Item::Leave) => "LEAVE LAN",
                None => "",
            };
            set_text(&mut t, menu_label(menu.sel == *i, label));
            // The modes wait for a player.
            let waiting = session.players() == 0 && matches!(items.get(*i), Some(Item::Classic | Item::City));
            color.0 = if waiting { DIM } else { INK };
        }
        if let Some(&CardLine(i, row)) = card {
            let (a, b) = &card_text[i];
            set_text(&mut t, if row == 0 { a.clone() } else { b.clone() });
            // The line that says to press A blinks.
            let base = if row == 0 { INK } else { DIM };
            let press = session.slots[i].is_none() && (if row == 0 { a } else { b }).starts_with("PRESS");
            color.0 = if press { SLOT_COLORS[i].with_alpha(blink) } else { base };
        }
    }
    for (card, mut frame) in &mut cards {
        let c = SLOT_COLORS[card.0];
        frame.color = if session.slots[card.0].is_some() { c } else { c.with_alpha(0.45) };
    }
}

/// `at` moved toward `goal` by up to `step`; exactly `goal` once there (a parked tank stays put).
fn step_toward(at: f32, goal: f32, step: f32) -> f32 {
    if at < goal { (at + step).min(goal) } else { (at - step).max(goal) }
}

/// How far off its parking spot a title tank is, in % of screen width: eased, so it slows as it parks.
fn tank_offset(slot: usize, at: f32) -> f32 {
    let away = (1.0 - at).powi(2) * 60.0;
    if slot == 0 { -away } else { away }
}

fn drive(
    time: Res<Time>,
    session: Res<Session>,
    gfx: Res<Gfx>,
    mut tanks: Query<(&mut TitleTank, &mut UiTransform, &mut ImageNode, &mut Visibility)>,
) {
    let dt = time.delta_secs();
    for (mut tank, mut tf, mut img, mut vis) in &mut tanks {
        let goal = if session.slots[tank.slot].is_some() { 1.0 } else { 0.0 };
        let before = tank.at;
        tank.at = step_toward(before, goal, dt / DRIVE);
        // Treads roll while it moves and rest on the still frame once it stops.
        let frame = if tank.at != before {
            tank.tread += dt;
            (tank.tread / 0.08) as usize % 2
        } else {
            tank.tread = 0.0;
            0
        };
        if img.image != gfx.players[3][frame] {
            img.image = gfx.players[3][frame].clone();
        }
        tf.translation.x = Val::Vw(tank_offset(tank.slot, tank.at));
        *vis = if tank.at > 0.0 { Visibility::Inherited } else { Visibility::Hidden };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_fits_and_drops_what_a_link_cant_share() {
        let all = crate::Platform { achieve: Some(|_, _| {}), ..default() };
        assert_eq!(items(false, &all).len(), 7, "the longest menu");
        assert_eq!(items(true, &all), vec![Item::Classic, Item::City, Item::Leave]);
        assert!(!items(false, &crate::Platform { lan: None, ..default() }).contains(&Item::Lan));
    }

    #[test]
    fn the_combo_opens_on_its_last_press_only() {
        use Key::*;
        let mut keys = Vec::new();
        let feed = |keys: &mut Vec<Key>, ks: &[Key]| ks.iter().map(|&k| combo(keys, k)).collect::<Vec<_>>();
        assert_eq!(feed(&mut keys, &COMBO), [false, false, false, false, false, true]);
        keys.clear();
        // A false start that overlaps the real one: the last six still count.
        let l = Go(Dir::Left);
        let r = Go(Dir::Right);
        let u = Go(Dir::Up);
        assert!(*feed(&mut keys, &[l, r, l, r, l, u, u, A]).last().unwrap());
        keys.clear();
        assert!(!feed(&mut keys, &[l, r, l, u, Go(Dir::Down), u, A]).contains(&true));
    }

    #[test]
    fn hints_name_keys_only_with_a_keyboard() {
        assert_eq!(hints(&crate::Platform::default()).len(), 2);
        assert_eq!(hints(&crate::Platform { keyboard: false, ..default() }).len(), 1);
        assert_eq!(empty_card(true, true).0, "CONTROLLER FOUND");
        assert_eq!(empty_card(false, false), ("PRESS A TO JOIN", ""));
    }

    #[test]
    fn tanks_drive_in_from_their_own_side_and_park() {
        assert_eq!(tank_offset(0, 1.0), 0.0);
        assert_eq!(tank_offset(1, 1.0), 0.0);
        assert!(tank_offset(0, 0.0) < -50.0, "player 1 waits off the left edge");
        assert!(tank_offset(1, 0.0) > 50.0, "player 2 off the right");
        assert_eq!(step_toward(1.0, 1.0, 0.1), 1.0, "a parked tank doesn't rock back and forth");
        assert_eq!(step_toward(0.0, 0.0, 0.1), 0.0);
        assert_eq!(step_toward(0.95, 1.0, 0.1), 1.0);
        assert_eq!(step_toward(0.5, 0.0, 0.1), 0.4);
        assert!(tank_offset(0, 0.9).abs() < (tank_offset(0, 0.1) - tank_offset(0, 0.0)).abs(), "slows as it parks");
    }
}
