//! VERSUS screens (ADR 0019) around the duel: VERSUS SETUP (map, NPC army points, rounds; either pad) and the
//! results after each round: who took it, round wins per player, tanks destroyed. A plays the next round, or a
//! rematch with the same setup once the match is decided; B goes back to the title.

use super::screens::{ScreenTime, advance_pressed};
use super::*;
use crate::game::versus::Match;
use crate::game::{Screen, Session, Sfx};
use crate::geom::Dir;
use crate::gfx::Gfx;
use crate::input::DeviceIntents;
use crate::rules::{ARMY_BUDGETS, MATCH_ROUNDS};

pub fn plugin(app: &mut App) {
    app.insert_resource(Setup { row: 0, map: 0, army: 2, rounds: 1 })
        .add_systems(OnEnter(Screen::VersusSetup), spawn_setup)
        .add_systems(Update, (setup_input, setup_refresh).chain().run_if(in_state(Screen::VersusSetup)))
        .add_systems(OnEnter(Screen::VersusResults), spawn)
        .add_systems(Update, input.run_if(in_state(Screen::VersusResults)));
}

/// The setup under the cursor, kept between matches: the row (MAP, NPC ARMY, ROUNDS) and each row's choice
/// (map 0 = RANDOM, else 1-based; indexes into `ARMY_BUDGETS` / `MATCH_ROUNDS`).
#[derive(Resource, Clone, Copy, PartialEq, Debug)]
struct Setup {
    row: usize,
    map: usize,
    army: usize,
    rounds: usize,
}

impl Setup {
    /// Up/down picks a row, left/right changes its choice; everything stops at its ends. True if anything moved.
    fn step(&mut self, dir: Dir, maps: usize) -> bool {
        let before = *self;
        match dir {
            Dir::Up => self.row = self.row.saturating_sub(1),
            Dir::Down => self.row = (self.row + 1).min(2),
            Dir::Left | Dir::Right => {
                let (v, last) = match self.row {
                    0 => (&mut self.map, maps),
                    1 => (&mut self.army, ARMY_BUDGETS.len() - 1),
                    _ => (&mut self.rounds, MATCH_ROUNDS.len() - 1),
                };
                *v = if dir == Dir::Left { v.saturating_sub(1) } else { (*v + 1).min(last) };
            }
        }
        *self != before
    }

    fn line(&self, row: usize) -> String {
        let s = match row {
            0 if self.map == 0 => "MAP   RANDOM".into(),
            0 => format!("MAP   {}", self.map),
            1 => match ARMY_BUDGETS[self.army] {
                0 => "NPC ARMY   OFF".into(),
                n => format!("NPC ARMY   {n} POINTS"),
            },
            _ => match MATCH_ROUNDS[self.rounds] {
                1 => "ROUNDS   1".into(),
                n => format!("ROUNDS   BEST OF {n}"),
            },
        };
        menu_label(row == self.row, &s)
    }
}

#[derive(Component)]
struct MapTile(usize);

fn spawn_setup(mut commands: Commands, gfx: Res<Gfx>, setup: Res<Setup>, mut heads: Headings) {
    commands.spawn((screen_root(), DespawnOnExit(Screen::VersusSetup))).with_children(|root| {
        root.spawn(heads.node("VERSUS", Block::Steel, 7.0));
        root.spawn(Node { height: vh(1), ..default() });
        root.spawn((text(setup.line(0), 3.6, INK), MenuLine(0)));
        root.spawn(Node { column_gap: vh(1.6), ..default() }).with_children(|row| {
            for i in 0..=crate::versus_map::count() {
                row.spawn((
                    Node {
                        width: vh(16),
                        height: vh(16),
                        border: UiRect::all(vh(0.5)),
                        border_radius: BorderRadius::all(vh(1.2)),
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BorderColor::all(DIM),
                    BackgroundColor(Color::srgb(0.06, 0.065, 0.08)),
                    Outline::new(vh(0.5), vh(0.5), Color::NONE),
                    MapTile(i),
                ))
                .with_children(|tile| {
                    if i == 0 {
                        tile.spawn(text("?", 9.0, GOLD));
                    } else {
                        tile.spawn((
                            ImageNode::new(gfx.versus_maps[i - 1].clone()),
                            Node { width: percent(100), height: percent(100), border_radius: BorderRadius::all(vh(0.7)), ..default() },
                        ));
                    }
                });
            }
        });
        root.spawn(Node { height: vh(1), ..default() });
        for row in 1..3 {
            root.spawn((text(setup.line(row), 3.6, INK), MenuLine(row)));
        }
        root.spawn(Node { height: vh(2), ..default() });
        root.spawn(text("up/down choose   left/right change   A start   B back", 2.4, DIM));
    });
}

fn setup_input(
    mut commands: Commands,
    devices: Res<DeviceIntents>,
    mut setup: ResMut<Setup>,
    mut next: ResMut<NextState<Screen>>,
    mut sfx: MessageWriter<Sfx>,
) {
    let nav = devices.0.iter().map(|(_, i)| *i).fold(Default::default(), crate::input::merge);
    if nav.nav.is_some_and(|d| setup.step(d, crate::versus_map::count())) {
        sfx.write(Sfx::MenuMove);
    }
    if nav.back {
        next.set(Screen::Play);
    } else if nav.confirm || nav.start {
        let budget = ARMY_BUDGETS[setup.army];
        commands.insert_resource(Match::new(setup.map, budget, MATCH_ROUNDS[setup.rounds]));
        sfx.write(Sfx::MenuPick);
        next.set(if budget > 0 { Screen::Draft } else { Screen::Stage });
    }
}

fn setup_refresh(
    time: Res<Time>,
    setup: Res<Setup>,
    mut lines: Query<(&MenuLine, &mut Text, &mut TextColor)>,
    mut tiles: Query<(&MapTile, &mut BorderColor, &mut Outline)>,
) {
    let pulse = 0.75 + 0.25 * (time.elapsed_secs() * 5.0).sin();
    for (line, mut t, mut color) in &mut lines {
        set_text(&mut t, setup.line(line.0));
        color.0 = if line.0 == setup.row { INK } else { DIM };
    }
    for (tile, mut border, mut outline) in &mut tiles {
        let chosen = tile.0 == setup.map;
        *border = BorderColor::all(if chosen { GOLD } else { DIM });
        outline.color = if chosen && setup.row == 0 { Color::WHITE.with_alpha(pulse) } else { Color::NONE };
    }
}

/// The round's end banner and the results heading.
pub fn head(last: Option<usize>) -> String {
    last.map_or("DRAW".into(), |p| format!("PLAYER {} WINS", p + 1))
}

/// The results' line under the heading: the round, or the match once it is decided.
fn subhead(duel: &Match) -> String {
    match duel.result() {
        Some(Some(_)) => format!("takes the match {} - {}", duel.wins[0], duel.wins[1]),
        Some(None) => format!("match drawn {} - {}", duel.wins[0], duel.wins[1]),
        None if duel.last.is_some() => format!("takes round {} (best of {})", duel.round, duel.rounds),
        None => format!("round {} drawn (best of {})", duel.round, duel.rounds),
    }
}

fn spawn(mut commands: Commands, session: Res<Session>, duel: Res<Match>, mut time: ResMut<ScreenTime>, mut heads: Headings) {
    time.0 = 0.0;
    let over = duel.result();
    commands.spawn((screen_root(), DespawnOnExit(Screen::VersusResults))).with_children(|r| {
        r.spawn(heads.node(&head(over.unwrap_or(duel.last)), Block::Gold, 8.0));
        r.spawn(text(subhead(&duel), 3.4, DIM));
        r.spawn(Node { height: vh(3), ..default() });
        r.spawn(Node { column_gap: vh(8), align_items: AlignItems::Center, ..default() }).with_children(|row| {
            for p in 0..2 {
                if p == 1 {
                    row.spawn(text("-", 9.0, DIM));
                }
                row.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: vh(1), ..default() })
                    .with_children(|c| {
                        c.spawn(text(format!("PLAYER {}", p + 1), 4.0, [GOLD, GREEN][p]));
                        c.spawn(text(duel.wins[p].to_string(), 9.0, INK));
                        let (frags, npcs) = session.slots[p].as_ref().map_or((0, 0), |s| (s.frags, s.kills.iter().sum::<u32>()));
                        c.spawn(text(format!("{frags} tanks destroyed"), 2.8, DIM));
                        if duel.budget > 0 {
                            c.spawn(text(format!("{npcs} NPCs destroyed"), 2.8, DIM));
                        }
                    });
            }
        });
        r.spawn(Node { height: vh(3), ..default() });
        r.spawn(text(if over.is_some() { "A rematch   B title" } else { "A next round   B title" }, 3.0, DIM));
    });
}

fn input(
    t: Res<Time>,
    mut time: ResMut<ScreenTime>,
    session: Res<Session>,
    mut duel: ResMut<Match>,
    devices: Res<DeviceIntents>,
    mut next: ResMut<NextState<Screen>>,
    mut sfx: MessageWriter<Sfx>,
) {
    if (0..2).any(|i| session.intent(i, &devices).back) {
        next.set(Screen::Title);
    } else if advance_pressed(&mut time, t.delta_secs(), &session, &devices) {
        sfx.write(Sfx::MenuPick);
        if duel.result().is_some() {
            duel.rematch();
            // Same setup; the armies go back to the draft so each player may change theirs.
            next.set(if duel.budget > 0 { Screen::Draft } else { Screen::Stage });
        } else {
            duel.round += 1;
            next.set(Screen::Stage);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heading_names_the_winner() {
        assert_eq!(head(Some(1)), "PLAYER 2 WINS");
        assert_eq!(head(None), "DRAW");
        let mut m = Match { last: Some(0), wins: [1, 0], ..Match::new(0, 0, 3) };
        assert_eq!(subhead(&m), "takes round 1 (best of 3)");
        m.round = 2;
        m.wins = [2, 0];
        assert_eq!(subhead(&m), "takes the match 2 - 0");
    }

    #[test]
    fn setup_rows_stop_at_their_ends() {
        let mut s = Setup { row: 0, map: 0, army: 2, rounds: 1 };
        assert!(!s.step(Dir::Left, 3), "RANDOM is the first map");
        assert!(!s.step(Dir::Up, 3));
        for _ in 0..5 {
            s.step(Dir::Right, 3);
        }
        assert_eq!(s.map, 3);
        assert_eq!(s.line(0), "> MAP   3 <");
        s.step(Dir::Down, 3);
        s.step(Dir::Left, 3);
        s.step(Dir::Left, 3);
        s.step(Dir::Left, 3);
        assert_eq!((s.army, s.line(1).as_str()), (0, "> NPC ARMY   OFF <"));
        s.step(Dir::Down, 3);
        assert!(!s.step(Dir::Down, 3));
        s.step(Dir::Right, 3);
        assert!(!s.step(Dir::Right, 3));
        assert_eq!(s.line(2), "> ROUNDS   BEST OF 5 <");
    }
}
