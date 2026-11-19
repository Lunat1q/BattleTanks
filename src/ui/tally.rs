//! Results / game over table and its classic count-up: rows one after another, a blip per kill, then the stage time
//! bonus and the score.

use super::screens::ScreenTime;
use super::*;
use crate::game::{Mode, Session, Sfx};
use crate::input::DeviceIntents;
use crate::rules::EnemyKind;

const KIND_NAMES: [&str; 4] = ["LIGHT ", "SCOUT ", "GUNNER", "HEAVY "];
/// Count-up pace: one kill per tick, a pause between rows.
pub const TICK: f32 = 0.07;
pub const ROW_PAUSE: f32 = 0.35;
/// Results move on by themselves this long after the tally finished.
pub const RESULTS_WAIT: f32 = 10.0;

/// The results table, re-rendered as the tally counts up.
#[derive(Component)]
pub struct Tally;

/// What the count-up shows `t` seconds in, for rows whose biggest count is `maxes[k]`: a cap per kind row (`None` =
/// not reached yet), whether the time bonus and score lines are up, and kills counted so far (one blip each).
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TallyView {
    pub caps: [Option<u32>; 4],
    pub bonus: bool,
    pub total: bool,
    pub ticks: u32,
}

pub fn tally_view(t: f32, maxes: [u32; 4]) -> TallyView {
    let mut start = 0.0;
    let mut view = TallyView { caps: [None; 4], bonus: false, total: false, ticks: 0 };
    for (k, &m) in maxes.iter().enumerate() {
        if t >= start {
            let n = (((t - start) / TICK) as u32 + 1).min(m);
            view.caps[k] = Some(n);
            view.ticks += n;
        }
        start += m as f32 * TICK + ROW_PAUSE;
    }
    view.bonus = t >= start;
    view.total = t >= start + ROW_PAUSE;
    view
}

/// Seconds the whole count-up takes.
pub fn tally_len(maxes: [u32; 4]) -> f32 {
    maxes.iter().map(|&m| m as f32 * TICK + ROW_PAUSE).sum::<f32>() + ROW_PAUSE
}

pub fn kill_maxes(session: &Session) -> [u32; 4] {
    let mut m = [0; 4];
    for s in session.slots.iter().flatten() {
        for (mk, sk) in m.iter_mut().zip(s.kills) {
            *mk = (*mk).max(sk);
        }
    }
    m
}

/// Results / game over table; `view` limits it to what the tally has counted so far (`None` = everything).
pub fn table(session: &Session, view: Option<TallyView>) -> String {
    let present: Vec<usize> = (0..2).filter(|&i| session.slots[i].is_some()).collect();
    let mut out = format!("{:8}", "");
    for &i in &present {
        out += &format!("{:>16}", format!("{}P", i + 1));
    }
    for k in EnemyKind::ALL {
        let cap = view.map_or(Some(u32::MAX), |v| v.caps[k.index()]);
        out += &format!("\n{:8}", if cap.is_some() { KIND_NAMES[k.index()] } else { "" });
        for &i in &present {
            let n = session.slots[i].as_ref().map_or(0, |s| s.kills[k.index()]);
            let cell = cap.map_or(String::new(), |c| format!("{:>2} x {:>3}", n.min(c), k.points()));
            out += &format!("{cell:>16}");
        }
    }
    // A classic clear carries a time bonus (each player gets it; it is already in the score); a City Run clear
    // shows its time (what the world board ranks).
    let cleared = session.log.last().filter(|l| l.won && view.is_some());
    out += "\n";
    if let Some(l) = cleared.filter(|_| view.is_some_and(|v| v.bonus)) {
        if session.mode == Mode::City {
            out += &format!("\n{:8}{:>16}", "TIME", crate::scores::clock_text(l.tenths));
        } else {
            out += &format!("\n{:8}", format!("{}:{:02}", l.tenths / 600, l.tenths / 10 % 60));
            for _ in &present {
                out += &format!("{:>16}", format!("+{} TIME", session.bonus));
            }
        }
    } else if cleared.is_some() {
        out += "\n";
    }
    let show_total = view.is_none_or(|v| v.total);
    out += &format!("\n{:8}", if show_total { "SCORE" } else { "" });
    for &i in &present {
        let score = session.slots[i].as_ref().map_or(0, |s| s.score);
        out += &format!("{:>16}", if show_total { score.to_string() } else { String::new() });
    }
    out
}

/// Count the results table up, a blip per kill; A while counting jumps to the end.
pub fn tally(
    t: Res<Time>,
    mut time: ResMut<ScreenTime>,
    session: Res<Session>,
    devices: Res<DeviceIntents>,
    mut heard: Local<u32>,
    mut q: Query<&mut Text, With<Tally>>,
    mut sfx: MessageWriter<Sfx>,
) {
    let len = tally_len(kill_maxes(&session));
    if time.0 == 0.0 {
        *heard = 0;
    }
    if time.0 < len && (0..2).any(|i| session.intent(i, &devices).confirm) {
        time.0 = len; // the press is used up here; `results_input` waits for the next one
    }
    let view = tally_view(time.0, kill_maxes(&session));
    if *heard != u32::MAX && (view.total || view.ticks > *heard) {
        sfx.write(if view.total { Sfx::MenuPick } else { Sfx::MenuMove });
        *heard = if view.total { u32::MAX } else { view.ticks };
    }
    if let Ok(mut text) = q.single_mut() {
        set_text(&mut text, table(&session, Some(view)));
    }
    // The results clock ticks here (not in `results_input`, which waits while names are entered).
    time.0 += t.delta_secs();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tally_counts_rows_one_after_another() {
        let maxes = [3, 0, 2, 1];
        let v = tally_view(0.0, maxes);
        assert_eq!((v.caps, v.bonus, v.total, v.ticks), ([Some(1), None, None, None], false, false, 1));
        let v = tally_view(3.0 * TICK + 2.0 * ROW_PAUSE + 0.01, maxes);
        assert_eq!(v.caps[..3], [Some(3), Some(0), Some(1)], "an empty row only pauses, then the next one starts");
        let end = tally_view(tally_len(maxes), maxes);
        assert_eq!((end.caps, end.bonus, end.total, end.ticks), ([Some(3), Some(0), Some(2), Some(1)], true, true, 6));
        assert!(!tally_view(tally_len(maxes) - ROW_PAUSE - 0.01, maxes).bonus);
    }
}
