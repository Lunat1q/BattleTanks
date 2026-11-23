//! VERSUS results (ADR 0019): who took the round, round wins per player, tanks destroyed. A next round (the next
//! map), B back to the title.

use super::screens::{ScreenTime, advance_pressed};
use super::*;
use crate::game::versus::Match;
use crate::game::{Screen, Session, Sfx};
use crate::input::DeviceIntents;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::VersusResults), spawn).add_systems(Update, input.run_if(in_state(Screen::VersusResults)));
}

/// The round's end banner and this screen's heading.
pub fn head(last: Option<usize>) -> String {
    last.map_or("DRAW".into(), |p| format!("PLAYER {} WINS", p + 1))
}

fn spawn(mut commands: Commands, session: Res<Session>, duel: Res<Match>, mut time: ResMut<ScreenTime>, mut heads: Headings) {
    time.0 = 0.0;
    commands.spawn((screen_root(), DespawnOnExit(Screen::VersusResults))).with_children(|r| {
        r.spawn(heads.node(&head(duel.last), Block::Gold, 8.0));
        r.spawn(text(format!("after round {}", duel.round), 3.4, DIM));
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
        r.spawn(text("A next round   B title", 3.0, DIM));
    });
}

fn input(
    t: Res<Time>,
    mut time: ResMut<ScreenTime>,
    mut session: ResMut<Session>,
    mut duel: ResMut<Match>,
    devices: Res<DeviceIntents>,
    mut next: ResMut<NextState<Screen>>,
    mut sfx: MessageWriter<Sfx>,
) {
    if (0..2).any(|i| session.intent(i, &devices).back) {
        next.set(Screen::Title);
    } else if advance_pressed(&mut time, t.delta_secs(), &session, &devices) {
        duel.round += 1;
        session.stage = session.stage % crate::versus_map::count() + 1;
        sfx.write(Sfx::MenuPick);
        next.set(Screen::Stage);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn heading_names_the_winner() {
        assert_eq!(super::head(Some(1)), "PLAYER 2 WINS");
        assert_eq!(super::head(None), "DRAW");
    }
}
