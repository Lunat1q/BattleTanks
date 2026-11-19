//! Scores: the local best (`scores.txt` next to the settings: hi score + last initials) and the world board on the
//! owner's score server (ADR 0009; game id `battletanks`). A classic run is sent at game over as its stage log; the
//! server recomputes and checks the score.
//! The game key is baked in at build time (`BT_SCORES_KEY`, from the gitignored `scores.key`); builds without it
//! stay offline.

use crate::game::{Mode, Phase, Screen, Session, StageLog, StageRun};
use bevy::prelude::*;
use std::sync::{Arc, Mutex};

pub const URL: &str = "https://scores.maxpodolski.com/api/games/battletanks/scores?format=text";
const KEY: Option<&str> = option_env!("BT_SCORES_KEY");
const FILE: &str = "scores.txt";

/// Local best and the initials last entered per slot.
#[derive(Resource, Clone, PartialEq, Debug)]
pub struct Best {
    pub hi: u32,
    pub names: [String; 2],
}

impl Best {
    pub fn parse(text: &str) -> Self {
        let get = |k: &str| text.lines().find_map(|l| l.trim().strip_prefix(k).map(str::to_string));
        let name = |k: &str, d: &str| get(k).map(|n| clean_name(&n)).filter(|n| !n.is_empty()).unwrap_or(d.into());
        Best { hi: get("hi=").and_then(|v| v.parse().ok()).unwrap_or(0), names: [name("name1=", "AAA"), name("name2=", "BBB")] }
    }
    pub fn serialize(&self) -> String {
        format!("hi={}\nname1={}\nname2={}\n", self.hi, self.names[0], self.names[1])
    }
}

/// Up to 3 letters A-Z, upper case; everything else dropped (the server checks the same).
pub fn clean_name(s: &str) -> String {
    s.chars().filter(char::is_ascii_alphabetic).map(|c| c.to_ascii_uppercase()).take(3).collect()
}

/// One board line.
#[derive(Clone, PartialEq, Debug)]
pub struct Row {
    pub rank: u32,
    pub score: u32,
    pub stages: u32,
    pub names: String,
}

/// The server's answer: our rank (`None` = beyond the kept top 1000), runs ever sent to this board, and the
/// top rows plus the ones around ours.
#[derive(Clone, PartialEq, Debug)]
pub struct Board {
    pub rank: Option<u32>,
    pub total: u32,
    pub rows: Vec<Row>,
}

/// Plain-text answer (no JSON parser in the game): `you <rank|0> <total>`, then `rank<TAB>score<TAB>progress<TAB>names`
/// (progress = stages cleared).
pub fn parse_board(text: &str) -> Option<Board> {
    let mut lines = text.lines();
    let head: Vec<u32> = lines.next()?.strip_prefix("you ")?.split(' ').map(|v| v.parse().ok()).collect::<Option<_>>()?;
    let [rank, total] = head[..] else { return None };
    let rows = lines
        .filter_map(|l| {
            let f: Vec<&str> = l.splitn(4, '\t').collect();
            let [r, s, st, n] = f[..] else { return None };
            Some(Row { rank: r.parse().ok()?, score: s.parse().ok()?, stages: st.parse().ok()?, names: n.to_string() })
        })
        .collect();
    Some(Board { rank: (rank > 0).then_some(rank), total, rows })
}

/// JSON body for a run. Names are already `clean_name`d, so no escaping is needed.
pub fn payload(names: &[String], score: u32, log: &[StageLog]) -> String {
    let list = |v: &[u32]| v.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
    let stages: Vec<String> = log
        .iter()
        .map(|s| {
            let kills: Vec<String> = s.kills.iter().map(|k| format!("[{}]", list(k))).collect();
            format!(
                r#"{{"stage":{},"tenths":{},"won":{},"kills":[{}],"pickups":[{}]}}"#,
                s.stage,
                s.tenths,
                s.won,
                kills.join(","),
                list(&s.pickups)
            )
        })
        .collect();
    let names: Vec<String> = names.iter().map(|n| format!("\"{n}\"")).collect();
    format!(
        r#"{{"version":"{}","players":{},"names":[{}],"score":{score},"stages":[{}]}}"#,
        env!("CARGO_PKG_VERSION"),
        names.len(),
        names.join(","),
        stages.join(",")
    )
}

/// Default `Post`: the `curl` that ships with Windows 10+ (and every desktop Linux/macOS), so the game needs no
/// HTTP/TLS crates. ponytail: a missing curl just means offline; swap in a client crate if that ever bites.
pub fn curl_post(url: &str, key: &str, body: &str) -> Result<String, String> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut cmd = Command::new("curl");
    cmd.args(["-sS", "--fail-with-body", "--max-time", "15", "-H", "Content-Type: application/json", "-H"])
        .arg(format!("X-Game-Key: {key}"))
        .args(["--data-binary", "@-", url])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    std::os::windows::process::CommandExt::creation_flags(&mut cmd, 0x0800_0000); // CREATE_NO_WINDOW
    let mut child = cmd.spawn().map_err(|e| format!("curl: {e}"))?;
    child.stdin.take().ok_or("curl stdin")?.write_all(body.as_bytes()).map_err(|e| e.to_string())?;
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if out.status.success() {
        Ok(text)
    } else {
        Err(if text.is_empty() { String::from_utf8_lossy(&out.stderr).trim().into() } else { text })
    }
}

/// The online board: idle, waiting for the worker, or its answer.
#[derive(Resource, Default)]
pub struct Online {
    pub pending: Option<Arc<Mutex<Option<Result<String, String>>>>>,
    pub result: Option<Result<Board, String>>,
}

impl Online {
    /// Sending works in this build and on this host.
    pub fn available(platform: &crate::Platform) -> bool {
        KEY.is_some() && platform.post_score.is_some()
    }

    pub fn send(&mut self, platform: &crate::Platform, body: String) {
        let (Some(key), Some(post)) = (KEY, platform.post_score) else { return };
        let slot = Arc::new(Mutex::new(None));
        let out = slot.clone();
        std::thread::spawn(move || {
            let r = post(URL, key, &body);
            if let Ok(mut o) = out.lock() {
                *o = Some(r);
            }
        });
        self.pending = Some(slot);
        self.result = None;
    }

    pub fn sending(&self) -> bool {
        self.pending.is_some()
    }
}

fn poll(mut online: ResMut<Online>) {
    let Some(r) = online.pending.as_ref().and_then(|p| p.lock().ok()?.take()) else { return };
    online.pending = None;
    online.result = Some(r.and_then(|t| parse_board(&t).ok_or_else(|| format!("bad answer: {t}"))));
    if let Some(Err(e)) = &online.result {
        warn!("score not sent: {e}");
    }
}

/// Leaving a classic stage (won, lost or quit) records it for the online score. After the end delay, so kills
/// in the last 3 s count too.
fn log_stage(mut session: ResMut<Session>, run: Res<StageRun>) {
    if session.mode != Mode::Classic {
        return;
    }
    let entry = StageLog {
        stage: session.stage,
        tenths: (run.clock * 10.0) as u32,
        won: run.phase == Phase::Won,
        kills: session.slots.iter().flatten().map(|s| s.kills).collect(),
        pickups: session.slots.iter().flatten().map(|s| s.pickups).collect(),
    };
    session.log.push(entry);
}

pub fn save(best: &Best) {
    let Some(p) = crate::settings::data_file(FILE) else { return };
    if let Err(e) = p.parent().map_or(Ok(()), std::fs::create_dir_all).and_then(|_| std::fs::write(&p, best.serialize())) {
        warn!("could not save scores to {}: {e}", p.display());
    }
}

/// A screen change after play: keep a new local best.
fn keep_best(session: Res<Session>, mut best: ResMut<Best>) {
    if session.hi_score > best.hi {
        best.hi = session.hi_score;
        save(&best);
    }
}

pub fn plugin(app: &mut App) {
    let best = crate::settings::data_file(FILE).and_then(|p| std::fs::read_to_string(p).ok()).map_or(Best::parse(""), |t| Best::parse(&t));
    app.world_mut().resource_mut::<Session>().hi_score = best.hi;
    app.insert_resource(best)
        .init_resource::<Online>()
        .add_systems(OnEnter(Screen::Results), keep_best)
        .add_systems(OnEnter(Screen::GameOver), keep_best)
        .add_systems(OnEnter(Screen::Title), keep_best)
        .add_systems(OnExit(Screen::Stage), log_stage)
        .add_systems(Update, poll);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn best_file_round_trips_and_cleans_names() {
        let b = Best::parse("hi=12300\nname1=ab1c\nname2=\n");
        assert_eq!(b, Best { hi: 12300, names: ["ABC".into(), "BBB".into()] });
        assert_eq!(Best::parse(&b.serialize()), b);
        assert_eq!(Best::parse("garbage").hi, 0);
    }

    #[test]
    fn board_answer_parses() {
        let b = parse_board("you 2 340\n1\t9000\t5\tMAX & ZED\n2\t8000\t4\tAAA\n").unwrap();
        assert_eq!((b.rank, b.total, b.rows.len()), (Some(2), 340, 2));
        assert_eq!(b.rows[0], Row { rank: 1, score: 9000, stages: 5, names: "MAX & ZED".into() });
        assert_eq!(parse_board("you 0 5\n").unwrap().rank, None);
        assert!(parse_board("<html>").is_none());
    }

    #[test]
    fn payload_is_the_stage_log() {
        let log = [StageLog { stage: 3, tenths: 905, won: true, kills: vec![[1, 2, 0, 0], [0, 0, 1, 4]], pickups: vec![1, 0] }];
        let body = payload(&["MAX".into(), "ZED".into()], 4200, &log);
        assert!(body.contains(r#""players":2,"names":["MAX","ZED"],"score":4200"#), "{body}");
        assert!(
            body.ends_with(r#""stages":[{"stage":3,"tenths":905,"won":true,"kills":[[1,2,0,0],[0,0,1,4]],"pickups":[1,0]}]}"#),
            "{body}"
        );
    }
}
