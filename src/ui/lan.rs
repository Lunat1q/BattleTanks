//! LAN CO-OP from the title: host a game (a beacon goes out under this machine's name) or join one heard on the
//! network. Once linked, both machines go to the title together (`net`). Driven by this machine's own devices.

use super::*;
use crate::Platform;
use crate::game::{Screen, Sfx};
use crate::geom::Dir;
use crate::input::{DeviceIntents, Intent, merge};
use crate::net::Net;

#[derive(Resource, Default)]
struct Sel(usize);

#[derive(Component)]
struct Body;

#[derive(Component)]
struct Note;

pub fn plugin(app: &mut App) {
    app.init_resource::<Sel>()
        .add_systems(OnEnter(Screen::Lan), (open, spawn))
        .add_systems(OnExit(Screen::Lan), close)
        .add_systems(Update, (input, refresh).chain().run_if(in_state(Screen::Lan)));
}

fn open(mut net: ResMut<Net>, platform: Res<Platform>, mut sel: ResMut<Sel>) {
    sel.0 = 0;
    net.name = platform.lan.map_or_else(|| "?".into(), |f| f(true));
    net.note.clear();
    net.listen();
}

fn close(mut net: ResMut<Net>, platform: Res<Platform>) {
    net.cancel();
    net.finder = None;
    if let Some(f) = platform.lan {
        f(false);
    }
}

fn spawn(mut commands: Commands, mut heads: Headings) {
    commands.spawn((screen_root(), DespawnOnExit(Screen::Lan))).with_children(|root| {
        root.spawn(heads.node("LAN CO-OP", Block::Water, 7.5));
        root.spawn(text("two players, one game, same Wi-Fi", 3.0, DIM));
        root.spawn(Node { height: vh(3), ..default() });
        root.spawn((text("", 4.4, INK), Node { flex_direction: FlexDirection::Column, ..default() }, Body));
        root.spawn(Node { height: vh(3), ..default() });
        root.spawn((text("", 3.0, RED), Note));
        root.spawn(text("one player picks HOST GAME, the other joins it  |  A pick  B back", 2.4, DIM));
    });
}

/// Menu entries: HOST GAME, one per host heard, BACK; or just CANCEL while hosting or joining.
fn entries(net: &Net) -> Vec<String> {
    if net.hosting() || net.joining() {
        return vec!["CANCEL".into()];
    }
    let hosts = net.finder.iter().flat_map(|f| &f.found).map(|f| format!("JOIN  {}", f.name.to_uppercase()));
    std::iter::once("HOST GAME".to_string()).chain(hosts).chain(std::iter::once("BACK".into())).collect()
}

fn input(
    devices: Res<DeviceIntents>,
    mut net: ResMut<Net>,
    mut sel: ResMut<Sel>,
    mut next: ResMut<NextState<Screen>>,
    mut sfx: MessageWriter<Sfx>,
) {
    let i = devices.0.iter().fold(Intent::default(), |a, (_, i)| merge(a, *i));
    let n = entries(&net).len();
    sel.0 = sel.0.min(n - 1);
    match i.nav {
        Some(Dir::Up) => sel.0 = (sel.0 + n - 1) % n,
        Some(Dir::Down) => sel.0 = (sel.0 + 1) % n,
        _ => {}
    }
    if i.nav.is_some() {
        sfx.write(Sfx::MenuMove);
    }
    let busy = net.hosting() || net.joining();
    if i.back || (i.confirm && busy) {
        sfx.write(Sfx::MenuMove);
        if busy {
            net.cancel();
            net.listen();
        } else {
            next.set(Screen::Title);
        }
    } else if i.confirm {
        sfx.write(Sfx::MenuPick);
        match sel.0 {
            0 => net.host(),
            s if s == n - 1 => next.set(Screen::Title),
            s => net.join(s - 1),
        }
        sel.0 = 0;
    }
}

fn refresh(net: Res<Net>, sel: Res<Sel>, mut body: Query<&mut Text, (With<Body>, Without<Note>)>, mut note: Query<&mut Text, With<Note>>) {
    let mut lines = Vec::new();
    if net.hosting() {
        lines.push(format!("HOSTING AS {}", net.name.to_uppercase()));
        lines.push("waiting for player 2...".into());
        lines.push(String::new());
    } else if net.joining() {
        lines.push("CONNECTING...".into());
        lines.push(String::new());
    }
    let items = entries(&net);
    lines.extend(items.iter().enumerate().map(|(i, s)| menu_label(sel.0 == i, s)));
    if items.len() == 2 {
        lines.insert(1, "(no games found yet)".into());
    }
    if let Ok(mut t) = body.single_mut() {
        set_text(&mut t, lines.join("\n"));
    }
    if let Ok(mut t) = note.single_mut() {
        set_text(&mut t, net.note.clone());
    }
}
