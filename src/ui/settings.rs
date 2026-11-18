//! Settings screen (from the title): FPS limit and VSync. Any device can drive it; changes save immediately.

use super::*;
use crate::game::{Screen, Sfx};
use crate::geom::Dir;
use crate::input::{DeviceIntents, Intent, merge};
use crate::settings::{self, Settings};

const ITEMS: usize = 3; // FPS LIMIT, VSYNC, BACK

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
        root.spawn(Node { height: vh(3), ..default() });
        for i in 0..ITEMS {
            root.spawn((text("", 5.0, INK), MenuLine(i)));
        }
        root.spawn(Node { height: vh(3), ..default() });
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
    match i.nav {
        Some(Dir::Up) => sel.0 = (sel.0 + ITEMS - 1) % ITEMS,
        Some(Dir::Down) => sel.0 = (sel.0 + 1) % ITEMS,
        Some(d @ (Dir::Left | Dir::Right)) if sel.0 == 0 => settings.cycle_fps(d == Dir::Right),
        Some(Dir::Left | Dir::Right) if sel.0 == 1 => settings.vsync = !settings.vsync,
        _ => {}
    }
    if i.confirm {
        match sel.0 {
            0 => settings.cycle_fps(true),
            1 => settings.vsync = !settings.vsync,
            _ => next.set(Screen::Title),
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

fn refresh(settings: Res<Settings>, sel: Res<Sel>, mut lines: Query<(&mut Text, &MenuLine)>) {
    for (mut t, MenuLine(i)) in &mut lines {
        let label = match i {
            0 if settings.fps_limit == 0 => "FPS LIMIT  < OFF >".to_string(),
            0 => format!("FPS LIMIT  < {} >", settings.fps_limit),
            1 => format!("VSYNC  < {} >", if settings.vsync { "ON" } else { "OFF" }),
            _ => "BACK".to_string(),
        };
        set_text(&mut t, menu_label(sel.0 == *i, &label));
    }
}
