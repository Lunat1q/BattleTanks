//! Achievement rows, shared by the ACHIEVEMENTS list and the NEW ACHIEVEMENTS screen: the pixel badge, the name,
//! the description (with a progress bar for counters still running) and the points with their gem. Rows are
//! spawned empty per slot; `Rows::show` fills one.

use super::*;
use crate::achieve::{LIST, Record};
use crate::gfx::Gfx;
use bevy::ecs::hierarchy::ChildSpawnerCommands;
use bevy::ecs::system::SystemParam;

/// Row height in vh.
pub const ROW_H: f32 = 8.8;
const LOCKED: Color = Color::srgb(0.70, 0.72, 0.78);
const TRACK: Color = Color::srgb(0.06, 0.06, 0.08);

/// On every entity of a row, so a screen's own queries can stay clear of them (`Without<RowPart>`).
#[derive(Component, Default)]
pub struct RowPart;

#[derive(Component)]
#[require(RowPart)]
pub struct RowBox(pub usize);
#[derive(Component)]
#[require(RowPart)]
pub struct RowIcon(pub usize);
#[derive(Component)]
#[require(RowPart)]
struct RowGem(usize);
#[derive(Component)]
#[require(RowPart)]
struct RowBar(usize);
#[derive(Component)]
#[require(RowPart)]
struct RowFill(usize);

#[derive(Clone, Copy, PartialEq)]
enum Part {
    Name,
    Desc,
    Pts,
}

#[derive(Component)]
#[require(RowPart)]
struct RowText(usize, Part);

/// Resting background of slot `i`: alternating bands like a ledger.
pub fn row_bg(i: usize) -> Color {
    if i.is_multiple_of(2) { Color::srgb(0.105, 0.11, 0.135) } else { Color::srgb(0.15, 0.155, 0.185) }
}

/// One empty row for slot `i`, `width` wide (hidden until shown).
pub fn spawn(p: &mut ChildSpawnerCommands, i: usize, width: Val) {
    let row = Node {
        width,
        height: vh(ROW_H),
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        padding: UiRect::axes(vh(1.4), vh(0.0)),
        column_gap: vh(2.0),
        border: UiRect::left(vh(0.6)),
        flex_shrink: 0.0,
        ..default()
    };
    p.spawn((row, BackgroundColor(row_bg(i)), BorderColor::all(Color::NONE), UiTransform::IDENTITY, Visibility::Hidden, RowBox(i)))
        .with_children(|r| {
            r.spawn((
                ImageNode::default(),
                Node { width: vh(7.6), height: vh(7.6), flex_shrink: 0.0, ..default() },
                UiTransform::IDENTITY,
                RowIcon(i),
            ));
            r.spawn(Node { flex_direction: FlexDirection::Column, flex_grow: 1.0, row_gap: vh(0.6), ..default() }).with_children(|t| {
                t.spawn((text("", 3.3, GOLD), RowText(i, Part::Name)));
                t.spawn((text("", 2.3, INK), RowText(i, Part::Desc)));
                t.spawn((Node { width: vh(32), height: vh(0.9), display: Display::None, ..default() }, BackgroundColor(TRACK), RowBar(i)))
                    .with_child((Node { width: percent(0), height: percent(100), ..default() }, BackgroundColor(GOLD), RowFill(i)));
            });
            r.spawn(Node { align_items: AlignItems::Center, column_gap: vh(1.0), flex_shrink: 0.0, ..default() }).with_children(|g| {
                g.spawn((ImageNode::default(), Node { width: vh(2.6), height: vh(2.6), ..default() }, RowGem(i)));
                g.spawn((text("", 3.6, GOLD), RowText(i, Part::Pts)));
            });
        });
}

#[derive(SystemParam)]
pub struct Rows<'w, 's> {
    gfx: Res<'w, Gfx>,
    pub boxes: Query<
        'w,
        's,
        (&'static RowBox, &'static mut BackgroundColor, &'static mut BorderColor, &'static mut Visibility, &'static mut UiTransform),
        With<RowPart>,
    >,
    pub icons: Query<'w, 's, (&'static RowIcon, &'static mut ImageNode, &'static mut UiTransform), (With<RowPart>, Without<RowBox>)>,
    gems: Query<'w, 's, (&'static RowGem, &'static mut ImageNode), (With<RowPart>, Without<RowIcon>)>,
    texts: Query<'w, 's, (&'static RowText, &'static mut Text, &'static mut TextColor), With<RowPart>>,
    bars: Query<'w, 's, (&'static RowBar, &'static mut Node), (With<RowPart>, Without<RowFill>)>,
    fills: Query<'w, 's, (&'static RowFill, &'static mut Node), (With<RowPart>, Without<RowBar>)>,
}

impl Rows<'_, '_> {
    /// Fills slot `slot` with entry `entry` of `LIST` (`None` hides the row).
    pub fn show(&mut self, slot: usize, entry: Option<usize>, rec: &Record) {
        for (b, mut bg, mut border, mut vis, _) in &mut self.boxes {
            if b.0 == slot {
                vis.set_if_neq(if entry.is_some() { Visibility::Inherited } else { Visibility::Hidden });
                bg.0 = row_bg(slot);
                let done = entry.is_some_and(|i| rec.done.contains(LIST[i].key));
                border.set_if_neq(BorderColor::all(if done { GOLD } else { Color::NONE }));
            }
        }
        let Some(i) = entry else { return };
        let a = &LIST[i];
        let done = rec.done.contains(a.key);
        let secret = !done && a.hidden;
        let counting = !done && !secret && a.steps > 0;
        for (r, mut img, _) in &mut self.icons {
            if r.0 == slot {
                let want = if secret { &self.gfx.badge_hidden } else { &self.gfx.badges[i][usize::from(!done)] };
                if img.image != *want {
                    img.image = want.clone();
                }
            }
        }
        for (r, mut img) in &mut self.gems {
            if r.0 == slot {
                if img.image != self.gfx.gem {
                    img.image = self.gfx.gem.clone();
                }
                img.color = if done { Color::WHITE } else { Color::srgb(0.42, 0.42, 0.48) };
            }
        }
        for (r, mut t, mut color) in &mut self.texts {
            if r.0 != slot {
                continue;
            }
            let (s, c) = match r.1 {
                Part::Name if secret => ("???".to_string(), DIM),
                Part::Name => (a.name.to_string(), if done { GOLD } else { LOCKED }),
                Part::Desc if secret => ("Hidden achievement - keep playing to find it.".to_string(), DIM),
                Part::Desc if counting => (format!("{}   {} / {}", a.desc, rec.steps_of(a), a.steps), DIM),
                Part::Desc => (a.desc.to_string(), if done { INK } else { DIM }),
                Part::Pts => (a.pts.to_string(), if done { GOLD } else { DIM }),
            };
            set_text(&mut t, s);
            color.0 = c;
        }
        for (r, mut node) in &mut self.bars {
            if r.0 == slot {
                let want = if counting { Display::Flex } else { Display::None };
                if node.display != want {
                    node.display = want;
                }
            }
        }
        for (r, mut node) in &mut self.fills {
            if r.0 == slot {
                let want = percent(100.0 * rec.steps_of(a) as f32 / a.steps.max(1) as f32);
                if node.width != want {
                    node.width = want;
                }
            }
        }
    }
}
