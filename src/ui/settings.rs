//! Settings screen (from the title). Rows come from `settings::ITEMS` plus BACK.
//! Any device can drive it; every change saves immediately.

use super::*;
use crate::game::{Screen, Sfx};
use crate::geom::Dir;
use crate::input::{DeviceIntents, Intent, merge};
use crate::settings::{self, ITEMS, Settings};

const ROWS: usize = ITEMS.len() + 1; // + BACK

#[derive(Resource, Default)]
struct Sel(usize);

pub fn plugin(app: &mut App) {
    app.init_resource::<Sel>()
        .add_systems(OnEnter(Screen::Settings), spawn)
        .add_systems(Update, (input, refresh).chain().run_if(in_state(Screen::Settings)));
}

fn spawn(mut commands: Commands, mut sel: ResMut<Sel>) {
    sel.0 = 0;
    commands.spawn((screen_root(), DespawnOnExit(Screen::Settings))).with_children(|root| {
        root.spawn(text("SETTINGS", 10.0, GOLD));
        root.spawn(Node { height: vh(2), ..default() });
        for i in 0..ROWS {
            root.spawn((text("", 4.4, INK), MenuLine(i)));
        }
        root.spawn(Node { height: vh(2), ..default() });
        root.spawn(text("left / right to change   B to go back", 2.6, DIM));
    });
}

fn input(
    devices: Res<DeviceIntents>,
    mut settings: ResMut<Settings>,
    mut sel: ResMut<Sel>,
    mut next: ResMut<NextState<Screen>>,
    mut sfx: MessageWriter<Sfx>,
) {
    let i: Intent = devices.0.iter().map(|(_, i)| *i).fold(Intent::default(), merge);
    let before = *settings;
    let back_row = sel.0 == ROWS - 1;
    match i.nav {
        Some(Dir::Up) => sel.0 = (sel.0 + ROWS - 1) % ROWS,
        Some(Dir::Down) => sel.0 = (sel.0 + 1) % ROWS,
        Some(d @ (Dir::Left | Dir::Right)) if !back_row => settings.adjust(sel.0, d == Dir::Right),
        _ => {}
    }
    if i.confirm {
        if back_row { next.set(Screen::Title) } else { settings.adjust(sel.0, true) }
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

fn refresh(settings: Res<Settings>, sel: Res<Sel>, mut lines: Query<(&mut Text, &mut TextColor, &MenuLine)>) {
    for (mut t, mut color, MenuLine(i)) in &mut lines {
        let label = match ITEMS.get(*i) {
            Some(name) => format!("{name}  < {} >", settings.value(*i)),
            None => "BACK".to_string(),
        };
        // Gold + leading marker: the usual "> x <" would clash with the value arrows.
        let selected = sel.0 == *i;
        set_text(&mut t, if selected { format!("> {label}") } else { label });
        color.0 = if selected { GOLD } else { INK };
    }
}
