//! Classic stage select: a grid of stage tiles, each with its mini-map. Stages open one by one as they are cleared
//! (`Progress::classic`); the cursor moves only over open tiles. A starts, B goes back to PLAY.

use super::*;
use crate::game::progress::Progress;
use crate::game::{Mode, Screen, Session, Sfx};
use crate::geom::Dir;
use crate::gfx::Gfx;
use crate::input::DeviceIntents;

const COLS: usize = 6;
const LOCKED: Color = Color::srgb(0.20, 0.22, 0.27);

#[derive(Resource)]
struct Sel(usize);

#[derive(Component)]
struct Tile(usize);
#[derive(Component)]
struct TileMap(usize);
#[derive(Component)]
struct TileLock(usize);
#[derive(Component)]
struct Info;

pub fn plugin(app: &mut App) {
    app.insert_resource(Sel(1))
        .add_systems(OnEnter(Screen::StageSelect), spawn)
        .add_systems(Update, (input, refresh).chain().run_if(in_state(Screen::StageSelect)));
}

fn count() -> usize {
    crate::levels::stage_count()
}

/// The cursor after a move, 1-based; `open` = highest open stage. Left / right step through the stages in order
/// (across rows), up / down jump a row. A move onto a locked tile or off the grid stays put.
fn step(sel: usize, dir: Dir, open: usize, count: usize) -> usize {
    let last = open.min(count);
    let to = match dir {
        Dir::Left => sel.checked_sub(1),
        Dir::Right => Some(sel + 1),
        Dir::Up => sel.checked_sub(COLS),
        Dir::Down => Some(sel + COLS),
    };
    to.filter(|&n| (1..=last).contains(&n)).unwrap_or(sel)
}

fn spawn(
    mut commands: Commands,
    gfx: Res<Gfx>,
    progress: Res<Progress>,
    settings: Res<crate::settings::Settings>,
    mut sel: ResMut<Sel>,
    mut heads: Headings,
) {
    sel.0 = progress.open(Mode::Classic, settings.easy_mode).min(count());
    commands.spawn((screen_root(), DespawnOnExit(Screen::StageSelect))).with_children(|root| {
        root.spawn(heads.node("CLASSIC", Block::Brick, 6.5));
        root.spawn(Node {
            display: Display::Grid,
            grid_template_columns: RepeatedGridTrack::auto(COLS as u16),
            column_gap: vh(1.6),
            row_gap: vh(1.6),
            ..default()
        })
        .with_children(|grid| {
            for n in 1..=count() {
                grid.spawn((
                    Node {
                        width: vh(12),
                        height: vh(12),
                        border: UiRect::all(vh(0.5)),
                        border_radius: BorderRadius::all(vh(1.2)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BorderColor::all(LOCKED),
                    BackgroundColor(Color::srgb(0.06, 0.065, 0.08)),
                    Outline::new(vh(0.5), vh(0.5), Color::NONE),
                    Tile(n),
                ))
                .with_children(|tile| {
                    tile.spawn((
                        ImageNode::new(gfx.stage_maps[n - 1].clone()),
                        Node { width: percent(100), height: percent(100), border_radius: BorderRadius::all(vh(0.7)), ..default() },
                        TileMap(n),
                    ));
                    // Stage number, bottom-left over the map.
                    tile.spawn((
                        Node { position_type: PositionType::Absolute, left: vh(0.4), bottom: vh(0.1), ..default() },
                        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.55)),
                        children![(text(n.to_string(), 2.8, INK), Node { margin: UiRect::horizontal(vh(0.5)), ..default() })],
                    ));
                    tile.spawn((lock(), TileLock(n)));
                });
            }
        });
        root.spawn((text("", 4.0, INK), Info));
        root.spawn(text("move to choose   A start   B back   -   clear a stage to open the next", 2.4, DIM));
    });
}

/// A padlock out of two nodes: the shackle (a rounded frame open at the bottom) on the body.
fn lock() -> impl Bundle {
    let metal = Color::srgb(0.62, 0.65, 0.72);
    (
        Node { position_type: PositionType::Absolute, flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() },
        children![
            (
                Node {
                    width: vh(2.6),
                    height: vh(2.2),
                    border: UiRect { left: vh(0.6), right: vh(0.6), top: vh(0.6), bottom: Val::ZERO },
                    border_radius: BorderRadius::top(vh(1.4)),
                    ..default()
                },
                BorderColor::all(metal),
            ),
            (Node { width: vh(4.2), height: vh(3.2), border_radius: BorderRadius::all(vh(0.5)), ..default() }, BackgroundColor(metal)),
        ],
    )
}

fn input(
    devices: Res<DeviceIntents>,
    progress: Res<Progress>,
    settings: Res<crate::settings::Settings>,
    mut session: ResMut<Session>,
    mut sel: ResMut<Sel>,
    mut next: ResMut<NextState<Screen>>,
    mut sfx: MessageWriter<Sfx>,
) {
    let nav = devices.0.iter().map(|(_, i)| *i).fold(Default::default(), crate::input::merge);
    if let Some(dir) = nav.nav {
        let to = step(sel.0, dir, progress.open(Mode::Classic, settings.easy_mode), count());
        if to != sel.0 {
            sel.0 = to;
            sfx.write(Sfx::MenuMove);
        }
    }
    if nav.back {
        next.set(Screen::Play);
    } else if (nav.confirm || nav.start) && session.players() > 0 {
        session.mode = Mode::Classic;
        session.stage = sel.0;
        sfx.write(Sfx::MenuPick);
        next.set(Screen::Stage);
    }
}

fn refresh(
    time: Res<Time>,
    progress: Res<Progress>,
    settings: Res<crate::settings::Settings>,
    sel: Res<Sel>,
    mut tiles: Query<(&Tile, &mut BorderColor, &mut Outline)>,
    mut maps: Query<(&TileMap, &mut ImageNode)>,
    mut locks: Query<(&TileLock, &mut Visibility)>,
    mut info: Query<&mut Text, With<Info>>,
) {
    let open = progress.open(Mode::Classic, settings.easy_mode);
    let pulse = 0.75 + 0.25 * (time.elapsed_secs() * 5.0).sin();
    for (tile, mut border, mut outline) in &mut tiles {
        *border = BorderColor::all(match tile.0 {
            n if n < open => GOLD,
            n if n == open => INK,
            _ => LOCKED,
        });
        outline.color = if tile.0 == sel.0 { Color::WHITE.with_alpha(pulse) } else { Color::NONE };
    }
    for (map, mut img) in &mut maps {
        img.color = if map.0 <= open { Color::WHITE } else { Color::srgb(0.22, 0.22, 0.26) };
    }
    for (lock, mut vis) in &mut locks {
        *vis = if lock.0 > open { Visibility::Inherited } else { Visibility::Hidden };
    }
    if let Ok(mut t) = info.single_mut() {
        let status = if sel.0 < open { "   CLEARED" } else { "" };
        set_text(&mut t, format!("STAGE {}{status}", sel.0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_stays_on_open_tiles() {
        assert_eq!(step(1, Dir::Left, 5, 30), 1, "nothing left of stage 1");
        assert_eq!(step(1, Dir::Up, 5, 30), 1);
        assert_eq!(step(4, Dir::Right, 5, 30), 5);
        assert_eq!(step(5, Dir::Right, 5, 30), 5, "6 is locked");
        assert_eq!(step(5, Dir::Down, 5, 30), 5, "11 is locked");
        assert_eq!(step(6, Dir::Right, 9, 30), 7, "right runs on into the next row");
        assert_eq!(step(9, Dir::Up, 9, 30), 3);
        assert_eq!(step(3, Dir::Down, 9, 30), 9);
        assert_eq!(step(30, Dir::Right, 31, 30), 30, "all cleared: the last tile is the end");
        assert_eq!(step(26, Dir::Down, 31, 30), 26);
    }
}
