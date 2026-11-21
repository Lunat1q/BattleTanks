//! ACHIEVEMENTS (from the title, hosts with `Platform::achieve`): points earned, a tab per group with its count, and
//! the group's badge rows (`badge_rows`) with a scrollbar. Left/right changes the tab, up/down scrolls, B goes back.

use super::badge_rows::{self, ROW_H, RowPart, Rows};
use super::*;
use crate::achieve::list::{TABS, total_pts};
use crate::achieve::{LIST, Record};
use crate::game::{Screen, Sfx};
use crate::geom::Dir;
use crate::input::{DeviceIntents, Intent, merge};

const VISIBLE: usize = 7;
const PANEL_W: f32 = 146.0;

/// The tab and scroll, kept between visits.
#[derive(Resource, Default)]
struct Browse {
    tab: usize,
    scroll: usize,
}

#[derive(Component)]
struct Tab(usize);
#[derive(Component)]
struct TabText(usize);
#[derive(Component)]
struct Thumb;

pub fn plugin(app: &mut App) {
    app.init_resource::<Browse>()
        .add_systems(OnEnter(Screen::Achievements), spawn)
        .add_systems(Update, (input, refresh).chain().run_if(in_state(Screen::Achievements)));
}

/// `LIST` indices in tab `tab`.
pub fn entries(tab: usize) -> Vec<usize> {
    (0..LIST.len()).filter(|&i| LIST[i].group == TABS[tab].0).collect()
}

/// Points earned so far.
pub fn earned(rec: &Record) -> u32 {
    LIST.iter().filter(|a| rec.done.contains(a.key)).map(|a| a.pts).sum()
}

fn spawn(mut commands: Commands, rec: Res<Record>, mut heads: Headings) {
    let root = Node { row_gap: vh(1.2), ..screen_root() };
    commands.spawn((root, DespawnOnExit(Screen::Achievements))).with_children(|root| {
        root.spawn(heads.node("ACHIEVEMENTS", Block::Gold, 7.0));
        root.spawn(text(
            format!("{} of {} points earned   |   {} / {} unlocked", earned(&rec), total_pts(), rec.done.len(), LIST.len()),
            3.0,
            INK,
        ));
        root.spawn(Node { column_gap: vh(3.6), margin: UiRect::top(vh(1.0)), ..default() }).with_children(|tabs| {
            for t in 0..TABS.len() {
                let tab = Node { border: UiRect::bottom(vh(0.5)), padding: UiRect::bottom(vh(0.5)), ..default() };
                tabs.spawn((tab, BorderColor::all(Color::NONE), Tab(t))).with_child((text("", 2.8, DIM), TabText(t)));
            }
        });
        root.spawn((Node { width: vh(PANEL_W + 2.0), height: px(2), ..default() }, BackgroundColor(Color::srgb(0.28, 0.29, 0.34))));
        root.spawn(Node { column_gap: vh(1.2), ..default() }).with_children(|panel| {
            panel.spawn(Node { flex_direction: FlexDirection::Column, height: vh(ROW_H * VISIBLE as f32), ..default() }).with_children(
                |list| {
                    for i in 0..VISIBLE {
                        badge_rows::spawn(list, i, vh(PANEL_W));
                    }
                },
            );
            let track = Node { width: vh(0.8), height: vh(ROW_H * VISIBLE as f32), ..default() };
            panel.spawn((track, BackgroundColor(Color::srgb(0.08, 0.08, 0.10)))).with_child((
                Node { position_type: PositionType::Absolute, width: percent(100), ..default() },
                BackgroundColor(Color::srgb(0.78, 0.62, 0.26)),
                Thumb,
            ));
        });
        root.spawn(text("left/right  category      up/down  scroll      B  back", 2.4, DIM));
    });
}

fn input(devices: Res<DeviceIntents>, mut browse: ResMut<Browse>, mut next: ResMut<NextState<Screen>>, mut sfx: MessageWriter<Sfx>) {
    let i: Intent = devices.0.iter().map(|(_, i)| *i).fold(Intent::default(), merge);
    if i.back {
        sfx.write(Sfx::MenuMove);
        next.set(Screen::Title);
        return;
    }
    let n = TABS.len();
    let last = entries(browse.tab).len().saturating_sub(VISIBLE);
    let before = (browse.tab, browse.scroll);
    match i.nav {
        Some(Dir::Left) => (browse.tab, browse.scroll) = ((browse.tab + n - 1) % n, 0),
        Some(Dir::Right) => (browse.tab, browse.scroll) = ((browse.tab + 1) % n, 0),
        Some(Dir::Up) => browse.scroll = browse.scroll.saturating_sub(1),
        Some(Dir::Down) => browse.scroll = (browse.scroll + 1).min(last),
        None => {}
    }
    if (browse.tab, browse.scroll) != before {
        sfx.write(Sfx::MenuMove);
    }
}

fn refresh(
    browse: Res<Browse>,
    rec: Res<Record>,
    mut rows: Rows,
    mut tabs: Query<(&Tab, &mut BorderColor), Without<RowPart>>,
    mut labels: Query<(&TabText, &mut Text, &mut TextColor), Without<RowPart>>,
    mut thumb: Query<(&mut Node, &mut Visibility), (With<Thumb>, Without<RowPart>)>,
) {
    let list = entries(browse.tab);
    for slot in 0..VISIBLE {
        rows.show(slot, list.get(browse.scroll + slot).copied(), &rec);
    }
    for (t, mut border) in &mut tabs {
        border.set_if_neq(BorderColor::all(if t.0 == browse.tab { GOLD } else { Color::NONE }));
    }
    for (t, mut s, mut color) in &mut labels {
        let got = entries(t.0).iter().filter(|&&i| rec.done.contains(LIST[i].key)).count();
        set_text(&mut s, format!("{}  {}/{}", TABS[t.0].1, got, entries(t.0).len()));
        color.0 = if t.0 == browse.tab { GOLD } else { DIM };
    }
    if let Ok((mut node, mut vis)) = thumb.single_mut() {
        let total = list.len().max(1) as f32;
        vis.set_if_neq(if list.len() > VISIBLE { Visibility::Inherited } else { Visibility::Hidden });
        node.height = percent(100.0 * (VISIBLE as f32 / total).min(1.0));
        node.top = percent(100.0 * browse.scroll as f32 / total);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tabs_cover_the_list_and_points_add_up() {
        assert_eq!((0..TABS.len()).map(|t| entries(t).len()).sum::<usize>(), LIST.len());
        let mut rec = Record::default();
        assert_eq!(earned(&rec), 0);
        rec.unlock("first_blood");
        rec.unlock("city_10");
        assert_eq!(earned(&rec), 45);
    }
}
