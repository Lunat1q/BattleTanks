//! World board screen (`Screen::Board`): after a classic game over, or after a City Run district clear's results,
//! when the run can go to the board. Each player sets a name of up to 7 letters with their own controller, prefilled
//! with the one they used last (up/down letter, left/right move, A send; in 2P both press A). B skips: nothing is
//! sent. The answer shows our rank, the top 3, the two rows above ours and the one below; A moves on.

use super::screens::ScreenTime;
use super::*;
use crate::game::{Mode, Screen, Session, Sfx};
use crate::geom::Dir;
use crate::input::DeviceIntents;
use crate::scores::{self, Best, NAME_LEN, Online};

#[derive(Component)]
struct OnlineText;
#[derive(Component)]
struct HintText;

/// Name entry; `open` while names are being set. A blank cell is `b' '`.
#[derive(Resource, Default)]
struct Entry {
    open: bool,
    slots: Vec<usize>,
    letters: Vec<[u8; NAME_LEN]>,
    cursor: Vec<usize>,
    ready: Vec<bool>,
}

/// What one cell cycles through: the letters, then blank (blanks are dropped from the name).
const CELL: &[u8; 27] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ ";

pub fn plugin(app: &mut App) {
    app.init_resource::<Entry>()
        .add_systems(OnEnter(Screen::Board), (open, spawn))
        .add_systems(Update, (input, show).chain().run_if(in_state(Screen::Board)));
}

/// The run just finished can go to the world board: a classic run at game over, a City Run district clear.
pub fn wants_board(session: &Session, platform: &crate::Platform) -> bool {
    Online::available(platform)
        && match session.mode {
            Mode::Classic => session.team_score() > 0 && !session.log.is_empty(),
            Mode::City => session.log.last().is_some_and(|l| l.won),
        }
}

/// After a run (and its NEW ACHIEVEMENTS): the world board if it is on for this run, else onward. `skip` for LAN runs
/// (name entry waits on a network answer, which the two machines would get at different ticks) and EASY MODE runs.
pub fn after_run(session: &Session, platform: &crate::Platform, skip: bool) -> Screen {
    if wants_board(session, platform) && !skip { Screen::Board } else { onward(session) }
}

/// Where the game goes after the board (or instead of it): the district map, or the title.
pub fn onward(session: &Session) -> Screen {
    if session.mode == Mode::City { Screen::CityMap } else { Screen::Title }
}

/// One cell step up or down: A-Z, blank, wrapping.
pub fn step_letter(c: u8, up: bool) -> u8 {
    let i = CELL.iter().position(|&x| x == c).unwrap_or(26);
    CELL[(i + if up { 1 } else { CELL.len() - 1 }) % CELL.len()]
}

/// The cells' letters as a name; all blank -> `TANK` (the server wants at least one letter).
pub fn entered_name(cells: &[u8]) -> String {
    let name = scores::clean_name(&String::from_utf8_lossy(cells));
    if name.is_empty() { "TANK".into() } else { name }
}

/// A saved name padded with blanks to the entry's cells.
fn prefill(name: &str) -> [u8; NAME_LEN] {
    let mut l = [b' '; NAME_LEN];
    for (d, c) in l.iter_mut().zip(name.bytes()) {
        *d = c;
    }
    l
}

/// Rows worth showing around `rank`: the top 3, the two above ours, ours, the one below (outside the top: the top 3).
pub fn shown(row: u32, rank: Option<u32>) -> bool {
    row <= 3 || rank.is_some_and(|r| row + 2 >= r && row <= r + 1)
}

fn open(mut entry: ResMut<Entry>, mut online: ResMut<Online>, session: Res<Session>, best: Res<Best>, mut time: ResMut<ScreenTime>) {
    time.0 = 0.0;
    online.result = None;
    let slots: Vec<usize> = (0..2).filter(|&i| session.slots[i].is_some()).collect();
    entry.open = true;
    entry.letters = slots.iter().map(|&i| prefill(&best.names[i])).collect();
    entry.cursor = vec![0; slots.len()];
    entry.ready = vec![false; slots.len()];
    entry.slots = slots;
}

fn spawn(mut commands: Commands, session: Res<Session>, mut heads: Headings) {
    let run = match (session.mode, session.log.last()) {
        (Mode::City, Some(l)) => format!("DISTRICT {}   TIME {}", l.stage, scores::clock_text(l.tenths)),
        _ => format!("SCORE {}", session.team_score()),
    };
    commands.spawn((screen_root(), DespawnOnExit(Screen::Board))).with_children(|r| {
        r.spawn(heads.node("WORLD BOARD", Block::Water, 7.0));
        r.spawn(text(run, 3.8, DIM));
        r.spawn((text("", 3.6, INK), OnlineText));
        r.spawn((text("", 3.0, DIM), HintText));
    });
}

fn input(
    mut entry: ResMut<Entry>,
    mut online: ResMut<Online>,
    mut best: ResMut<Best>,
    session: Res<Session>,
    devices: Res<DeviceIntents>,
    platform: Res<crate::Platform>,
    t: Res<Time>,
    mut time: ResMut<ScreenTime>,
    mut next: ResMut<NextState<Screen>>,
    mut sfx: MessageWriter<Sfx>,
) {
    // A short guard so the press that left the previous screen can't send at once.
    time.0 += t.delta_secs();
    if time.0 < 0.4 {
        return;
    }
    if !entry.open {
        let pressed = entry.slots.iter().any(|&s| {
            let it = session.intent(s, &devices);
            it.confirm || it.start || it.back
        });
        if pressed && !online.sending() {
            sfx.write(Sfx::MenuPick);
            next.set(onward(&session));
        }
        return;
    }
    for k in 0..entry.slots.len() {
        let it = session.intent(entry.slots[k], &devices);
        if it.back {
            sfx.write(Sfx::MenuMove); // B: skip, nothing is sent
            next.set(onward(&session));
            return;
        }
        if entry.ready[k] {
            continue;
        }
        let c = entry.cursor[k];
        match it.nav {
            Some(Dir::Up) => entry.letters[k][c] = step_letter(entry.letters[k][c], true),
            Some(Dir::Down) => entry.letters[k][c] = step_letter(entry.letters[k][c], false),
            Some(Dir::Left) => entry.cursor[k] = c.saturating_sub(1),
            Some(Dir::Right) => entry.cursor[k] = (c + 1).min(NAME_LEN - 1),
            None => {}
        }
        entry.ready[k] = it.confirm || it.start;
        if it.nav.is_some() || entry.ready[k] {
            sfx.write(if entry.ready[k] { Sfx::MenuPick } else { Sfx::MenuMove });
        }
    }
    if entry.ready.iter().all(|&r| r) {
        entry.open = false;
        let names: Vec<String> = entry.letters.iter().map(|l| entered_name(l)).collect();
        for (&slot, name) in entry.slots.iter().zip(&names) {
            best.names[slot] = name.clone();
        }
        scores::save(&best);
        let body = match (session.mode, session.log.last()) {
            (Mode::City, Some(district)) => scores::city_payload(&names, district),
            _ => scores::payload(&names, session.team_score(), &session.log),
        };
        online.send(&platform, body);
    }
}

fn show(
    entry: Res<Entry>,
    online: Res<Online>,
    session: Res<Session>,
    mut text: Query<&mut Text, With<OnlineText>>,
    mut hint: Query<&mut Text, (With<HintText>, Without<OnlineText>)>,
) {
    let (body, help) = if entry.open {
        let names: Vec<String> = entry
            .slots
            .iter()
            .enumerate()
            .map(|(k, &slot)| {
                let letters: String = (0..NAME_LEN)
                    .map(|i| {
                        let ch = if entry.letters[k][i] == b' ' { '_' } else { entry.letters[k][i] as char };
                        if entry.cursor[k] == i && !entry.ready[k] { format!("[{ch}]") } else { format!(" {ch} ") }
                    })
                    .collect();
                format!("{}P {letters}{}", slot + 1, if entry.ready[k] { " OK" } else { "   " })
            })
            .collect();
        (format!("enter your name\n\n{}", names.join("\n")), "up/down letter   left/right move   A send   B skip")
    } else if online.sending() {
        ("sending score...".to_string(), "")
    } else {
        match &online.result {
            Some(Ok(b)) => (board_text(b, session.players(), (session.mode == Mode::City).then_some(session.stage)), "press A"),
            Some(Err(_)) => ("offline - score not sent".to_string(), "press A"),
            None => (String::new(), "press A"),
        }
    };
    if let Ok(mut t) = text.single_mut() {
        set_text(&mut t, body);
    }
    if let Ok(mut t) = hint.single_mut() {
        set_text(&mut t, help.to_string());
    }
}

/// Rank headline and the rows worth showing (`shown`); ours marked with `>`. `district` = a City Run board (scores
/// are clear times in tenths).
pub fn board_text(b: &scores::Board, players: usize, district: Option<usize>) -> String {
    let head = match b.rank {
        Some(r) => format!("WORLD RANK #{r} OF {}", b.total),
        None => format!("OUTSIDE THE TOP 1000 ({} RUNS)", b.total),
    };
    let board = district.map_or(format!("{players}P BOARD"), |d| format!("DISTRICT {d} {players}P FASTEST"));
    let mut out = format!("{head}\n{board}\n");
    let mut last = 0;
    for row in b.rows.iter().filter(|r| shown(r.rank, b.rank)) {
        if row.rank > last + 1 {
            out += "        ...\n";
        }
        last = row.rank;
        let mark = if Some(row.rank) == b.rank { ">" } else { " " };
        let score = if district.is_some() { scores::clock_text(row.score) } else { row.score.to_string() };
        out += &format!("\n{mark}{:>4}  {:<17} {score:>7}", row.rank, row.names);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scores::{Board, Row};

    #[test]
    fn letters_wrap() {
        assert_eq!(step_letter(b'Z', true), b' ');
        assert_eq!(step_letter(b' ', true), b'A');
        assert_eq!(step_letter(b'A', false), b' ');
        assert_eq!(step_letter(b'M', true), b'N');
    }

    #[test]
    fn names_drop_blanks_and_prefill_pads() {
        assert_eq!(entered_name(b"MAX    "), "MAX");
        assert_eq!(entered_name(b"M A X  "), "MAX");
        assert_eq!(entered_name(b"       "), "TANK");
        assert_eq!(&prefill("MAXIMUS"), b"MAXIMUS");
        assert_eq!(&prefill("ZED"), b"ZED    ");
    }

    #[test]
    fn board_shows_top_three_two_above_and_one_below() {
        let row = |rank| Row { rank, score: 1000 - rank, stages: 1, names: format!("R{rank}") };
        // The server sends the top 5 and 2 either side of ours.
        let b = Board { rank: Some(12), total: 40, rows: [1, 2, 3, 4, 5, 10, 11, 12, 13, 14].map(row).to_vec() };
        let t = board_text(&b, 2, None);
        assert!(t.starts_with("WORLD RANK #12 OF 40\n2P BOARD"), "{t}");
        let ranks: Vec<u32> = b.rows.iter().map(|r| r.rank).filter(|&r| shown(r, b.rank)).collect();
        assert_eq!(ranks, [1, 2, 3, 10, 11, 12, 13]);
        assert!(t.contains("...") && t.contains(">  12  R12") && !t.contains("R14") && !t.contains("R4 "), "{t}");
        assert!(!shown(4, None) && shown(3, None));
        assert!(board_text(&Board { rank: None, total: 5000, rows: vec![] }, 1, None).starts_with("OUTSIDE THE TOP 1000 (5000 RUNS)"));
        let row = Row { rank: 1, score: 1503, stages: 30, names: "MAX".into() };
        let city = board_text(&Board { rank: Some(1), total: 3, rows: vec![row] }, 1, Some(4));
        assert!(city.contains("DISTRICT 4 1P FASTEST") && city.contains(">   1  MAX                2:30.3"), "{city}");
    }
}
