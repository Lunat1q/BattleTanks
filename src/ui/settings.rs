//! Settings screen (from the title). Rows come from `settings::shown_items` plus BACK.
//! Any device can drive it; every change saves immediately. Turning EASY MODE on asks first (A yes, B no).

use super::*;
use crate::Platform;
use crate::game::{Screen, Sfx};
use crate::geom::Dir;
use crate::input::{DeviceIntents, Intent, merge};
use crate::settings::{self, ITEMS, Settings};

#[derive(Resource, Default)]
struct Sel(usize);

/// The EASY MODE question is up.
#[derive(Resource, Default)]
struct Asking(bool);

#[derive(Component)]
struct Popup;

const EASY_TEXT: &str = "EASY MODE\n\nLosing a tank costs only one star, not all.\nTwo more spare tanks, a longer shield\nafter a respawn.\n\nStages and districts opened in easy mode\nopen only for easy mode. Easy runs don't go\nto the world board and earn no achievements.\nYour hi score still counts.\n\nA  turn it on        B  keep it off";

/// A settings change that turns EASY MODE on waits for the player's yes; everything else applies at once.
fn needs_asking(before: &Settings, after: &Settings) -> bool {
    !before.easy_mode && after.easy_mode
}

/// `ITEMS` indices on screen, in order; BACK comes after them.
fn rows(platform: &Platform) -> Vec<usize> {
    settings::shown_items(platform.host_window)
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Sel>()
        .init_resource::<Asking>()
        .add_systems(OnEnter(Screen::Settings), spawn)
        .add_systems(Update, (input, refresh).chain().run_if(in_state(Screen::Settings)));
}

fn spawn(mut commands: Commands, mut sel: ResMut<Sel>, mut asking: ResMut<Asking>, platform: Res<Platform>, mut heads: Headings) {
    let n = rows(&platform).len();
    sel.0 = 0;
    asking.0 = false;
    commands.spawn((screen_root(), DespawnOnExit(Screen::Settings))).with_children(|root| {
        root.spawn(heads.node("SETTINGS", Block::Trees, 8.0));
        root.spawn(Node { height: vh(2), ..default() });
        for i in 0..=n {
            root.spawn((text("", 4.0, INK), MenuLine(i)));
        }
        root.spawn(Node { height: vh(2), ..default() });
        root.spawn(text("left / right to change   B to go back", 2.6, DIM));
    });
    commands
        .spawn((
            Node { position_type: PositionType::Absolute, ..screen_root() },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            GlobalZIndex(10),
            Visibility::Hidden,
            Popup,
            DespawnOnExit(Screen::Settings),
        ))
        .with_children(|root| {
            root.spawn((
                Node { padding: UiRect::all(vh(3)), border: UiRect::all(px(3)), ..default() },
                BackgroundColor(Color::srgb(0.10, 0.11, 0.14)),
                BorderColor::all(GOLD),
            ))
            .with_child((text(EASY_TEXT, 3.4, INK), TextLayout::justify(Justify::Center)));
        });
}

fn input(
    devices: Res<DeviceIntents>,
    mut settings: ResMut<Settings>,
    mut sel: ResMut<Sel>,
    mut asking: ResMut<Asking>,
    platform: Res<Platform>,
    mut next: ResMut<NextState<Screen>>,
    mut sfx: MessageWriter<Sfx>,
) {
    let i: Intent = devices.0.iter().map(|(_, i)| *i).fold(Intent::default(), merge);
    if asking.0 {
        if i.confirm {
            settings.easy_mode = true;
            settings::save(&settings);
            sfx.write(Sfx::MenuPick);
        }
        if i.confirm || i.back {
            asking.0 = false;
            sfx.write(Sfx::MenuMove);
        }
        return;
    }
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
    if needs_asking(&before, &settings) {
        settings.easy_mode = false;
        asking.0 = true;
    }
    if *settings != before {
        settings::save(&settings);
    }
}

fn refresh(
    settings: Res<Settings>,
    sel: Res<Sel>,
    asking: Res<Asking>,
    platform: Res<Platform>,
    mut lines: Query<(&mut Text, &mut TextColor, &MenuLine)>,
    mut popup: Query<&mut Visibility, With<Popup>>,
) {
    for mut v in &mut popup {
        v.set_if_neq(if asking.0 { Visibility::Inherited } else { Visibility::Hidden });
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_turning_easy_mode_on_asks() {
        let off = Settings::default();
        let on = Settings { easy_mode: true, ..off };
        assert!(needs_asking(&off, &on));
        assert!(!needs_asking(&on, &off), "turning it off needs no yes");
        assert!(!needs_asking(&off, &Settings { shake: !off.shake, ..off }));
    }
}
