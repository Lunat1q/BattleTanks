//! Game over, classic mode: each player enters 3 letters with their own controller, then the run goes to the world
//! board (`scores`) and the answer shows beside the tally: our rank, the top 5 and the rows around ours.

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
        .add_systems(Update, (input, show).chain().run_if(in_state(Screen::GameOver)));
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

fn open(mut entry: ResMut<Entry>, mut online: ResMut<Online>, session: Res<Session>, best: Res<Best>, platform: Res<crate::Platform>) {
    online.result = None;
    let slots: Vec<usize> = (0..2).filter(|&i| session.slots[i].is_some()).collect();
    entry.open = session.mode == Mode::Classic && Online::available(&platform) && session.team_score() > 0 && !session.log.is_empty();
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
    mut sfx: MessageWriter<Sfx>,
) {
    if !entry.open {
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
        online.send(&platform, scores::payload(&names, session.team_score(), &session.log));
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
            Some(Ok(b)) => (board_text(b, session.players()), "press A"),
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

/// Rank headline and the rows the server picked; ours marked with `>`.
pub fn board_text(b: &scores::Board, players: usize) -> String {
    let head = match b.rank {
        Some(r) => format!("WORLD RANK #{r} OF {}", b.total),
        None => format!("OUTSIDE THE TOP 1000 ({} RUNS)", b.total),
    };
    let mut out = format!("{head}\n{}P BOARD\n", players);
    let mut last = 0;
    for row in &b.rows {
        if row.rank > last + 1 {
            out += "        ...\n";
        }
        last = row.rank;
        let mark = if Some(row.rank) == b.rank { ">" } else { " " };
        out += &format!("\n{mark}{:>4}  {:<9} {:>7}", row.rank, row.names, row.score);
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
        let t = board_text(&b, 2);
        assert!(t.starts_with("WORLD RANK #12 OF 40\n2P BOARD"), "{t}");
        assert!(t.contains("...") && t.contains(">  12  MAX & ZED"), "{t}");
        assert!(board_text(&Board { rank: None, total: 5000, rows: vec![] }, 1).starts_with("OUTSIDE THE TOP 1000 (5000 RUNS)"));
    }
}
