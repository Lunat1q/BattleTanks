//! Screens and HUD (Bevy UI). Sizes use viewport-height units so text reads well on a TV.

mod city_map;
mod hud;
mod screens;
mod settings;
mod title;

use bevy::prelude::*;
use bevy::text::FontSize;

pub const GOLD: Color = Color::srgb(1.0, 0.80, 0.32);
pub const GREEN: Color = Color::srgb(0.42, 0.92, 0.52);
pub const INK: Color = Color::srgb(0.92, 0.93, 0.96);
pub const DIM: Color = Color::srgb(0.55, 0.58, 0.66);
pub const RED: Color = Color::srgb(1.0, 0.36, 0.30);

pub fn plugin(app: &mut App) {
    app.add_plugins((title::plugin, hud::plugin, screens::plugin, settings::plugin, city_map::plugin));
}

/// Text bundle sized in % of screen height.
pub fn text(s: impl Into<String>, vh: f32, color: Color) -> impl Bundle {
    (Text::new(s), TextFont { font_size: FontSize::Vh(vh), ..default() }, TextColor(color))
}

/// Full-screen centred column.
pub fn screen_root() -> Node {
    Node {
        width: percent(100),
        height: percent(100),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        row_gap: vh(1.6),
        ..default()
    }
}

pub fn menu_label(selected: bool, label: &str) -> String {
    if selected { format!("> {label} <") } else { label.to_string() }
}

/// Text of a menu line, rewritten when the selection changes.
#[derive(Component)]
pub struct MenuLine(pub usize);

pub fn set_text(t: &mut Text, s: String) {
    if t.0 != s {
        t.0 = s;
    }
}
