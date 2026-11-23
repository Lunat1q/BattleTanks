//! Screens and HUD (Bevy UI). Sizes use viewport-height units so text reads well on a TV.

mod achievements;
mod badge_rows;
mod board;
mod bunker;
mod city_map;
mod draft;
mod hiscores;
mod hud;
mod lan;
mod play;
mod screens;
mod settings;
mod stages;
mod tally;
pub mod title;
mod unlocks;
mod versus;

pub use crate::gfx::Block;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::text::FontSize;
use std::collections::HashMap;

pub const GOLD: Color = Color::srgb(1.0, 0.80, 0.32);
pub const GREEN: Color = Color::srgb(0.42, 0.92, 0.52);
pub const INK: Color = Color::srgb(0.92, 0.93, 0.96);
pub const DIM: Color = Color::srgb(0.55, 0.58, 0.66);
pub const RED: Color = Color::srgb(1.0, 0.36, 0.30);

pub fn plugin(app: &mut App) {
    app.init_resource::<HeadingCache>().add_plugins((
        title::plugin,
        hud::plugin,
        screens::plugin,
        settings::plugin,
        city_map::plugin,
        board::plugin,
        hiscores::plugin,
        lan::plugin,
        achievements::plugin,
        unlocks::plugin,
        stages::plugin,
        bunker::plugin,
        play::plugin,
        versus::plugin,
        draft::plugin,
    ));
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

/// Block-built headings (game plan M12), painted on first use and kept for the session.
#[derive(Resource, Default)]
pub struct HeadingCache(HashMap<(String, Block), Handle<Image>>);

/// Pixels per unit of a heading: a 7 vh heading at 1080p shows ~1.3 px per unit, so this stays sharp up to 4K.
const HEADING_K: f32 = 2.5;

#[derive(SystemParam)]
pub struct Headings<'w> {
    images: ResMut<'w, Assets<Image>>,
    cache: ResMut<'w, HeadingCache>,
}

impl Headings<'_> {
    pub fn image(&mut self, s: &str, m: Block) -> Handle<Image> {
        let images = &mut self.images;
        self.cache.0.entry((s.to_string(), m)).or_insert_with(|| images.add(crate::gfx::block_text(&[(s, m)], HEADING_K))).clone()
    }

    /// `s` built from `m`, `h` vh tall (the width follows).
    pub fn node(&mut self, s: &str, m: Block, h: f32) -> impl Bundle + use<> {
        (ImageNode::new(self.image(s, m)), Node { height: vh(h), ..default() })
    }
}
