//! World board entry: at a classic game over, or after a City Run district clear (once the tally is counted), each
//! player enters 3 letters with their own controller, then the run goes to the world board (`scores`) and the answer
//! shows beside the tally: our rank, the top 5 and the rows around ours.

use super::screens::ScreenTime;
use super::tally::{kill_maxes, tally_len};
use super::*;
use crate::game::{Mode, Screen, Session, Sfx};
use crate::geom::Dir;
use crate::input::DeviceIntents;
use crate::scores::{self, Best, Online};

/// The board column on the game over screen.
#[derive(Component)]
pub struct OnlineText;
/// The "press A" line under it: says what A / B do right now.
#[derive(Component)]
pub struct HintText;

/// Name entry in progress. `cursor[k] == 3` = player `k` is done.
#[derive(Resource, Default)]
pub struct Entry {
    pub open: bool,
    slots: Vec<usize>,
    letters: Vec<[u8; 3]>,
    cursor: Vec<usize>,
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Entry>()
        .add_systems(OnEnter(Screen::GameOver), open)
        .add_systems(OnEnter(Screen::Results), open)
        .add_systems(Update, (input, show).chain().run_if(|s: Res<State<Screen>>| matches!(s.get(), Screen::GameOver | Screen::Results)));
}

/// Game over may advance: not while names are being entered.
pub fn entry_closed(entry: Res<Entry>) -> bool {
    !entry.open
}

/// One letter step up or down, A-Z wrapping.
pub fn step_letter(c: u8, up: bool) -> u8 {
    let i = (c.saturating_sub(b'A') as i32 + if up { 1 } else { 25 }) % 26;
    b'A' + i as u8
}

fn open(
    mut entry: ResMut<Entry>,
    mut online: ResMut<Online>,
    session: Res<Session>,
    best: Res<Best>,
    platform: Res<crate::Platform>,
    screen: Res<State<Screen>>,
) {
    online.result = None;
    let slots: Vec<usize> = (0..2).filter(|&i| session.slots[i].is_some()).collect();
    // Classic: the whole run at game over. City Run: each district clear, ranked by time.
    let run = match (session.mode, screen.get()) {
        (Mode::Classic, Screen::GameOver) => session.team_score() > 0 && !session.log.is_empty(),
        (Mode::City, Screen::Results) => session.log.last().is_some_and(|l| l.won),
        _ => false,
    };
    entry.open = run && Online::available(&platform);
    entry.letters = slots
        .iter()
        .map(|&i| {
            let mut l = *b"AAA";
            for (d, c) in l.iter_mut().zip(best.names[i].bytes()) {
                *d = c;
            }
            l
        })
        .collect();
    entry.cursor = vec![0; slots.len()];
    entry.slots = slots;
}

pub(super) fn input(
    mut entry: ResMut<Entry>,
    mut online: ResMut<Online>,
    mut best: ResMut<Best>,
    session: Res<Session>,
    devices: Res<DeviceIntents>,
    platform: Res<crate::Platform>,
    screen: Res<State<Screen>>,
    time: Res<ScreenTime>,
    mut sfx: MessageWriter<Sfx>,
) {
    if !entry.open || counting(&screen, &time, &session) {
        return;
    }
    for k in 0..entry.slots.len() {
        let it = session.intent(entry.slots[k], &devices);
        if it.back {
            entry.open = false; // B: don't send this run
            sfx.write(Sfx::MenuMove);
            return;
        }
        let c = entry.cursor[k];
        if c >= 3 {
            continue;
        }
        match it.nav {
            Some(Dir::Up) => entry.letters[k][c] = step_letter(entry.letters[k][c], true),
            Some(Dir::Down) => entry.letters[k][c] = step_letter(entry.letters[k][c], false),
            Some(Dir::Left) => entry.cursor[k] = c.saturating_sub(1),
            Some(Dir::Right) => entry.cursor[k] = c + 1,
            None => {}
        }
        if it.confirm || it.start {
            entry.cursor[k] = c + 1;
        }
        if it.nav.is_some() || it.confirm || it.start {
            sfx.write(if entry.cursor[k] >= 3 { Sfx::MenuPick } else { Sfx::MenuMove });
        }
    }
    if entry.cursor.iter().all(|&c| c >= 3) {
        entry.open = false;
        let names: Vec<String> = entry.letters.iter().map(|l| String::from_utf8_lossy(l).into_owned()).collect();
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

/// On results the count-up comes first (its first A skips it); the name entry waits until it is done.
fn counting(screen: &State<Screen>, time: &ScreenTime, session: &Session) -> bool {
    *screen.get() == Screen::Results && time.0 < tally_len(kill_maxes(session))
}

fn show(
    entry: Res<Entry>,
    online: Res<Online>,
    session: Res<Session>,
    screen: Res<State<Screen>>,
    time: Res<ScreenTime>,
    mut text: Query<&mut Text, With<OnlineText>>,
    mut hint: Query<&mut Text, (With<HintText>, Without<OnlineText>)>,
) {
    let (body, help) = if counting(&screen, &time, &session) {
        (String::new(), "press A")
    } else if entry.open {
        let names: Vec<String> = entry
            .slots
            .iter()
            .enumerate()
            .map(|(k, &slot)| {
                let letters: String = (0..3)
                    .map(|i| {
                        let ch = entry.letters[k][i] as char;
                        if entry.cursor[k] == i { format!("[{ch}]") } else { format!(" {ch} ") }
                    })
                    .collect();
                format!("{}P  {letters}{}", slot + 1, if entry.cursor[k] >= 3 { "  OK" } else { "" })
            })
            .collect();
        (format!("WORLD BOARD\nenter your name\n\n{}", names.join("\n")), "up/down letter   A next   B don't send")
    } else if online.sending() {
        ("WORLD BOARD\n\nsending score...".to_string(), "press A")
    } else {
        match &online.result {
            Some(Ok(b)) => (board_text(b, session.players(), (session.mode == Mode::City).then_some(session.stage)), "press A"),
            Some(Err(_)) => ("WORLD BOARD\n\noffline - score not sent".to_string(), "press A"),
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

/// Rank headline and the rows the server picked; ours marked with `>`. `district` = a City Run board (scores are
/// clear times in tenths).
pub fn board_text(b: &scores::Board, players: usize, district: Option<usize>) -> String {
    let head = match b.rank {
        Some(r) => format!("WORLD RANK #{r} OF {}", b.total),
        None => format!("OUTSIDE THE TOP 1000 ({} RUNS)", b.total),
    };
    let board = district.map_or(format!("{players}P BOARD"), |d| format!("DISTRICT {d} {players}P FASTEST"));
    let mut out = format!("{head}\n{board}\n");
    let mut last = 0;
    for row in &b.rows {
        if row.rank > last + 1 {
            out += "        ...\n";
        }
        last = row.rank;
        let mark = if Some(row.rank) == b.rank { ">" } else { " " };
        let score = if district.is_some() { scores::clock_text(row.score) } else { row.score.to_string() };
        out += &format!("\n{mark}{:>4}  {:<9} {score:>7}", row.rank, row.names);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scores::{Board, Row};

    #[test]
    fn letters_wrap() {
        assert_eq!(step_letter(b'Z', true), b'A');
        assert_eq!(step_letter(b'A', false), b'Z');
        assert_eq!(step_letter(b'M', true), b'N');
    }

    #[test]
    fn board_marks_our_row_and_gaps() {
        let row = |rank, names: &str| Row { rank, score: 1000 - rank, stages: 1, names: names.into() };
        let b = Board { rank: Some(12), total: 40, rows: vec![row(1, "TOP"), row(11, "ABC"), row(12, "MAX & ZED")] };
        let t = board_text(&b, 2, None);
        assert!(t.starts_with("WORLD RANK #12 OF 40\n2P BOARD"), "{t}");
        assert!(t.contains("...") && t.contains(">  12  MAX & ZED"), "{t}");
        assert!(board_text(&Board { rank: None, total: 5000, rows: vec![] }, 1, None).starts_with("OUTSIDE THE TOP 1000 (5000 RUNS)"));
        let row = Row { rank: 1, score: 1503, stages: 30, names: "MAX".into() };
        let city = board_text(&Board { rank: Some(1), total: 3, rows: vec![row] }, 1, Some(4));
        assert!(city.contains("DISTRICT 4 1P FASTEST") && city.contains(">   1  MAX        2:30.3"), "{city}");
    }
}
