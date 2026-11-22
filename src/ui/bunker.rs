//! The secret bunker (a combo on the title, `title::COMBO`): CHANGELOG (the game's CHANGELOG.md), OST (every music
//! track: A plays it, A again pauses / resumes, then the jukebox runs on down the list) and SOUND TEST (every sound effect). Left/right tab,
//! up/down move, B back. Getting in unlocks "Look Both Ways".

use super::*;
use crate::achieve::Record;
use crate::game::{Screen, Sfx};
use crate::geom::Dir;
use crate::input::{DeviceIntents, Intent, merge};
use crate::music::{self, Jukebox};
use crate::settings::Settings;
use std::sync::LazyLock;

const ROWS: usize = 14;
const TABS: [&str; 3] = ["CHANGELOG", "OST", "SOUND TEST"];

/// The tab, and per tab the picked line and the first line shown; kept between visits.
#[derive(Resource, Default)]
struct Browse {
    tab: usize,
    sel: [usize; 3],
    scroll: [usize; 3],
}

#[derive(Component)]
struct Row(usize);
#[derive(Component)]
struct TabText(usize);
#[derive(Component)]
struct Footer;

pub fn plugin(app: &mut App) {
    // `BT_TAB`: the smoke hook's tab.
    let tab = std::env::var("BT_TAB").ok().and_then(|t| t.parse().ok()).filter(|&t| t < TABS.len()).unwrap_or(0);
    app.insert_resource(Browse { tab, ..default() })
        .add_systems(OnEnter(Screen::Bunker), spawn)
        .add_systems(OnExit(Screen::Bunker), |mut j: ResMut<Jukebox>| *j = Jukebox::default())
        .add_systems(Update, (input, refresh).chain().run_if(in_state(Screen::Bunker)));
}

/// CHANGELOG.md as screen lines, `true` = a version heading; a blank line before every version but the first.
fn changelog(md: &str) -> Vec<(bool, String)> {
    let mut out = Vec::new();
    for l in md.lines() {
        if let Some(v) = l.strip_prefix("## ") {
            if !out.is_empty() {
                out.push((false, String::new()));
            }
            out.push((true, v.to_string()));
        } else if let Some(n) = l.strip_prefix("- ") {
            out.push((false, format!("  - {n}")));
        }
    }
    out
}

static CHANGES: LazyLock<Vec<(bool, String)>> = LazyLock::new(|| changelog(include_str!("../../CHANGELOG.md")));

/// `ArmorHit` -> `ARMOR HIT`.
fn spaced(name: &str) -> String {
    name.chars()
        .enumerate()
        .flat_map(|(i, c)| (i > 0 && c.is_uppercase()).then_some(' ').into_iter().chain([c.to_ascii_uppercase()]))
        .collect()
}

/// Tab `tab`'s lines.
fn lines(tab: usize) -> Vec<String> {
    match tab {
        0 => CHANGES.iter().map(|(_, s)| s.clone()).collect(),
        1 => music::NAMES.iter().enumerate().map(|(i, (name, at))| format!("{:>2}  {name:<17} {at}", i + 1)).collect(),
        _ => crate::audio::ALL.iter().map(|s| spaced(&format!("{s:?}"))).collect(),
    }
}

/// First line shown so that line `sel` is on screen, moving as little as possible from `scroll`.
fn follow(scroll: usize, sel: usize) -> usize {
    scroll.min(sel).max((sel + 1).saturating_sub(ROWS))
}

fn spawn(mut commands: Commands, mut heads: Headings, mut rec: ResMut<Record>, platform: Res<crate::Platform>) {
    // Only on hosts with achievements; `achieve::mirror` saves it and tells the host.
    let found = platform.achieve.is_some() && rec.unlock("look_both_ways");
    let root = Node { row_gap: vh(1.2), ..screen_root() };
    commands.spawn((root, DespawnOnExit(Screen::Bunker))).with_children(|root| {
        root.spawn(heads.node("BUNKER", Block::Steel, 7.0));
        if found {
            root.spawn(text("ACHIEVEMENT UNLOCKED: LOOK BOTH WAYS", 3.0, GOLD));
        } else {
            root.spawn(text("you found the secret bunker", 3.0, DIM));
        }
        root.spawn(Node { column_gap: vh(5), margin: UiRect::top(vh(1.0)), ..default() }).with_children(|tabs| {
            for t in 0..TABS.len() {
                tabs.spawn((text("", 3.2, DIM), TabText(t)));
            }
        });
        root.spawn((Node { width: vh(150), height: px(2), ..default() }, BackgroundColor(Color::srgb(0.28, 0.29, 0.34))));
        root.spawn(Node { flex_direction: FlexDirection::Column, width: vh(150), height: vh(ROWS as f32 * 3.4), ..default() })
            .with_children(|list| {
                for i in 0..ROWS {
                    list.spawn(Node { height: vh(3.4), ..default() }).with_child((text("", 2.6, INK), Row(i)));
                }
            });
        root.spawn((text("", 2.4, DIM), Footer));
    });
}

fn input(
    devices: Res<DeviceIntents>,
    mut b: ResMut<Browse>,
    mut next: ResMut<NextState<Screen>>,
    mut sfx: MessageWriter<Sfx>,
    mut jukebox: ResMut<Jukebox>,
) {
    let i: Intent = devices.0.iter().map(|(_, i)| *i).fold(Intent::default(), merge);
    if i.back {
        sfx.write(Sfx::MenuMove);
        next.set(Screen::Title);
        return;
    }
    let t = b.tab;
    let n = lines(t).len();
    let before = (t, b.sel[t], b.scroll[t]);
    match (i.nav, t) {
        (Some(Dir::Left), _) => b.tab = (t + TABS.len() - 1) % TABS.len(),
        (Some(Dir::Right), _) => b.tab = (t + 1) % TABS.len(),
        // The changelog only scrolls; the lists move a cursor and the view follows it.
        (Some(Dir::Up), 0) => b.scroll[0] = b.scroll[0].saturating_sub(1),
        (Some(Dir::Down), 0) => b.scroll[0] = (b.scroll[0] + 1).min(n.saturating_sub(ROWS)),
        (Some(Dir::Up), _) => b.sel[t] = (b.sel[t] + n - 1) % n,
        (Some(Dir::Down), _) => b.sel[t] = (b.sel[t] + 1) % n,
        _ => {}
    }
    if t > 0 {
        b.scroll[t] = follow(b.scroll[t], b.sel[t]);
    }
    if (b.tab, b.sel[t], b.scroll[t]) != before {
        sfx.write(Sfx::MenuMove);
    }
    if i.confirm {
        match t {
            1 => jukebox.press(b.sel[1]),
            2 => {
                sfx.write(crate::audio::ALL[b.sel[2]]);
            }
            _ => {}
        }
    }
}

fn refresh(
    b: Res<Browse>,
    jukebox: Res<Jukebox>,
    settings: Res<Settings>,
    mut rows: Query<(&Row, &mut Text, &mut TextColor), (Without<TabText>, Without<Footer>)>,
    mut tabs: Query<(&TabText, &mut Text, &mut TextColor), Without<Footer>>,
    mut footer: Query<&mut Text, With<Footer>>,
) {
    let t = b.tab;
    let all = lines(t);
    for (row, mut s, mut color) in &mut rows {
        let i = b.scroll[t] + row.0;
        let Some(line) = all.get(i) else {
            set_text(&mut s, String::new());
            continue;
        };
        let (picked, playing) = (t > 0 && i == b.sel[t], t == 1 && jukebox.0 == Some(i));
        let mark = if picked {
            "> "
        } else if t > 0 {
            "  "
        } else {
            ""
        };
        let state = match (playing, jukebox.1) {
            (false, _) => "",
            (true, false) => "   PLAYING",
            (true, true) => "   PAUSED",
        };
        set_text(&mut s, format!("{mark}{line}{state}"));
        color.0 = match t {
            0 if CHANGES[i].0 => GOLD,
            0 => INK,
            _ if playing => GOLD,
            _ if picked => INK,
            _ => DIM,
        };
    }
    for (tab, mut s, mut color) in &mut tabs {
        set_text(&mut s, menu_label(tab.0 == t, TABS[tab.0]));
        color.0 = if tab.0 == t { GOLD } else { DIM };
    }
    if let Ok(mut s) = footer.single_mut() {
        let hint = match t {
            0 => "up/down  scroll      left/right  tab      B  back",
            1 if settings.music == 0 => "MUSIC is off in SETTINGS - turn it up to listen",
            1 => "A  play / pause      up/down  pick      left/right  tab      B  back",
            _ => "A  play      up/down  pick      left/right  tab      B  back",
        };
        set_text(&mut s, hint.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changelog_reads_versions_and_notes() {
        let l = changelog("# Changelog\n\nintro\n\n## 0.2.0 (x)\n- b\n\n## 0.1.0 (y)\n- a\n");
        assert_eq!(
            l,
            [
                (true, "0.2.0 (x)".into()),
                (false, "  - b".into()),
                (false, String::new()),
                (true, "0.1.0 (y)".into()),
                (false, "  - a".into())
            ]
        );
        let top = CHANGES.iter().find(|l| l.0).unwrap();
        assert!(top.1.starts_with(env!("CARGO_PKG_VERSION")), "CHANGELOG.md opens with this version: {}", top.1);
        assert!(CHANGES.iter().all(|(_, s)| s.len() <= 92), "notes fit a row");
    }

    #[test]
    fn lists_follow_the_cursor() {
        assert_eq!(follow(0, 5), 0);
        assert_eq!(follow(0, ROWS), 1, "one past the bottom scrolls one");
        assert_eq!(follow(10, 3), 3, "above the top jumps up to it");
        assert_eq!(follow(5, 23), 23 + 1 - ROWS, "wrapping to the end shows the end");
        assert_eq!(spaced("ArmorHit"), "ARMOR HIT");
        assert_eq!(lines(1).len(), 24);
    }
}
