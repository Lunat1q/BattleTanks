//! Settings screen (from the title). Rows come from `settings::shown_items` plus BACK.
//! Any device can drive it; every change saves immediately.

use super::*;
use crate::Platform;
use crate::game::{Screen, Sfx};
use crate::geom::Dir;
use crate::input::{DeviceIntents, Intent, merge};
use crate::settings::{self, ITEMS, Settings};

#[derive(Resource, Default)]
struct Sel(usize);

/// `ITEMS` indices on screen, in order; BACK comes after them.
fn rows(platform: &Platform) -> Vec<usize> {
    settings::shown_items(platform.host_window)
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Sel>()
        .add_systems(OnEnter(Screen::Settings), spawn)
        .add_systems(Update, (input, refresh).chain().run_if(in_state(Screen::Settings)));
}

fn spawn(mut commands: Commands, mut sel: ResMut<Sel>, platform: Res<Platform>) {
    let n = rows(&platform).len();
    sel.0 = 0;
    commands.spawn((screen_root(), DespawnOnExit(Screen::Settings))).with_children(|root| {
        root.spawn(text("SETTINGS", 10.0, GOLD));
        root.spawn(Node { height: vh(2), ..default() });
        for i in 0..=n {
            root.spawn((text("", 4.0, INK), MenuLine(i)));
        }
        root.spawn(Node { height: vh(2), ..default() });
        root.spawn(text("left / right to change   B to go back", 2.6, DIM));
    });
}

fn input(
    devices: Res<DeviceIntents>,
    mut settings: ResMut<Settings>,
    mut sel: ResMut<Sel>,
    platform: Res<Platform>,
    mut next: ResMut<NextState<Screen>>,
    mut sfx: MessageWriter<Sfx>,
) {
    let i: Intent = devices.0.iter().map(|(_, i)| *i).fold(Intent::default(), merge);
    let before = *settings;
    let rows = rows(&platform);
    let n = rows.len() + 1;
    let item = rows.get(sel.0).copied();
    match (i.nav, item) {
        (Some(Dir::Up), _) => sel.0 = (sel.0 + n - 1) % n,
        (Some(Dir::Down), _) => sel.0 = (sel.0 + 1) % n,
        (Some(d @ (Dir::Left | Dir::Right)), Some(k)) => settings.adjust(k, d == Dir::Right),
        _ => {}
    }
    if i.confirm {
        match item {
            Some(k) => settings.adjust(k, true),
            None => next.set(Screen::Title),
        }
    }
    if i.back {
        next.set(Screen::Title);
    }
    if i.nav.is_some() || i.confirm || i.back {
        sfx.write(Sfx::MenuMove);
    }
    if *settings != before {
        settings::save(&settings);
    }
}

fn refresh(settings: Res<Settings>, sel: Res<Sel>, platform: Res<Platform>, mut lines: Query<(&mut Text, &mut TextColor, &MenuLine)>) {
    let rows = rows(&platform);
    for (mut t, mut color, MenuLine(i)) in &mut lines {
        let label = match rows.get(*i) {
            Some(&k) => format!("{}  < {} >", ITEMS[k], settings.value(k)),
            None => "BACK".to_string(),
        };
        // Gold + leading marker: the usual "> x <" would clash with the value arrows.
        let selected = sel.0 == *i;
        set_text(&mut t, if selected { format!("> {label}") } else { label });
        color.0 = if selected { GOLD } else { INK };
    }
}
