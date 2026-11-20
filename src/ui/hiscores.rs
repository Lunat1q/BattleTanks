//! HI SCORES (from the title): the world board's top 10 for any category, read live from the score server.
//! Up/down picks a selector (MODE, PLAYERS, DISTRICT in City Run), left/right or A changes it, B goes back.
//! Each board is fetched once per visit on a worker thread (`Platform::fetch_board`) and kept while you browse.

use super::*;
use crate::game::{Screen, Sfx};
use crate::geom::Dir;
use crate::input::{DeviceIntents, Intent, merge};
use crate::rules::CITY_DISTRICTS;
use crate::scores::{self, Best, Board, Row};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Rows asked of the server.
const TOP: usize = 10;
const SILVER: Color = Color::srgb(0.80, 0.84, 0.90);
const BRONZE: Color = Color::srgb(0.86, 0.58, 0.34);

/// What the screen shows; kept between visits so you come back to the board you last looked at.
#[derive(Resource, Clone, Copy, PartialEq, Debug)]
pub struct Pick {
    focus: usize,
    city: bool,
    players: usize,
    district: usize,
}

impl Default for Pick {
    fn default() -> Self {
        Pick { focus: 0, city: false, players: 1, district: 1 }
    }
}

impl Pick {
    /// The server's board id: `1p` / `2p`, or `city<d>-<p>p`.
    pub fn board(&self) -> String {
        if self.city { format!("city{}-{}p", self.district, self.players) } else { format!("{}p", self.players) }
    }

    /// Selector rows on screen: MODE, PLAYERS, and DISTRICT in City Run.
    pub fn rows(&self) -> usize {
        if self.city { 3 } else { 2 }
    }

    /// One D-pad press. Returns whether anything moved.
    pub fn step(&mut self, d: Dir) -> bool {
        let before = *self;
        let n = self.rows();
        let fwd = d == Dir::Right;
        match d {
            Dir::Up => self.focus = (self.focus + n - 1) % n,
            Dir::Down => self.focus = (self.focus + 1) % n,
            _ => match self.focus {
                0 => self.city = !self.city,
                1 => self.players = 3 - self.players,
                _ => {
                    self.district =
                        if fwd { self.district % CITY_DISTRICTS + 1 } else { (self.district + CITY_DISTRICTS - 2) % CITY_DISTRICTS + 1 }
                }
            },
        }
        *self != before
    }
}

/// One board: waiting for the worker, or its answer.
enum Fetch {
    Waiting(Arc<Mutex<Option<Result<String, String>>>>),
    Done(Result<Board, String>),
}

#[derive(Resource, Default)]
struct Boards(HashMap<String, Fetch>);

#[derive(Component)]
struct Head;
#[derive(Component)]
struct RowLine(usize);
#[derive(Component)]
struct Status;
#[derive(Component)]
struct Footer;

pub fn plugin(app: &mut App) {
    app.init_resource::<Pick>()
        .init_resource::<Boards>()
        .add_systems(OnEnter(Screen::HiScores), (|mut b: ResMut<Boards>| b.0.clear(), spawn))
        .add_systems(Update, (input, fetch, refresh).chain().run_if(in_state(Screen::HiScores)));
}

fn spawn(mut commands: Commands) {
    let root = Node { row_gap: vh(1.0), ..screen_root() };
    commands.spawn((root, DespawnOnExit(Screen::HiScores))).with_children(|root| {
        root.spawn(text("HI SCORES", 8.0, GOLD));
        root.spawn(text("world top 10", 2.6, DIM));
        root.spawn(Node { height: vh(1), ..default() });
        for i in 0..3 {
            root.spawn((text("", 3.8, INK), MenuLine(i)));
        }
        root.spawn(Node { height: vh(1), ..default() });
        // Fixed height, so the selectors above don't jump while a board loads.
        let table =
            Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: vh(0.5), height: vh(48), ..default() };
        root.spawn(table).with_children(|t| {
            t.spawn((text("", 3.2, DIM), Head));
            t.spawn((text("", 3.4, DIM), Status));
            for i in 0..TOP {
                t.spawn((text("", 3.2, INK), RowLine(i)));
            }
        });
        root.spawn((text("", 2.6, DIM), Footer));
        root.spawn(text("up/down pick   left/right change   B back", 2.4, DIM));
    });
}

fn input(
    devices: Res<DeviceIntents>,
    mut pick: ResMut<Pick>,
    mut boards: ResMut<Boards>,
    mut next: ResMut<NextState<Screen>>,
    mut sfx: MessageWriter<Sfx>,
) {
    let i: Intent = devices.0.iter().map(|(_, i)| *i).fold(Intent::default(), merge);
    if i.back {
        sfx.write(Sfx::MenuMove);
        next.set(Screen::Title);
        return;
    }
    if let Some(d) = i.nav
        && pick.step(d)
    {
        sfx.write(Sfx::MenuMove);
    }
    if i.confirm {
        // A on a board that failed: try again. Otherwise A changes the selector, like the settings screen.
        let id = pick.board();
        if matches!(boards.0.get(&id), Some(Fetch::Done(Err(_)))) {
            boards.0.remove(&id);
        } else {
            pick.step(Dir::Right);
        }
        sfx.write(Sfx::MenuPick);
    }
}

/// Asks for the shown board if we don't have it, and collects answers from the workers.
fn fetch(pick: Res<Pick>, mut boards: ResMut<Boards>, platform: Res<crate::Platform>) {
    let id = pick.board();
    if let (false, Some(get)) = (boards.0.contains_key(&id), platform.fetch_board) {
        let slot = Arc::new(Mutex::new(None));
        let out = slot.clone();
        let url = format!("{}&take={TOP}&board={id}", scores::URL);
        std::thread::spawn(move || {
            let r = get(&url);
            if let Ok(mut o) = out.lock() {
                *o = Some(r);
            }
        });
        boards.0.insert(id, Fetch::Waiting(slot));
    }
    for f in boards.0.values_mut() {
        if let Fetch::Waiting(slot) = f
            && let Some(r) = slot.lock().ok().and_then(|mut o| o.take())
        {
            let r = r.and_then(|t| scores::parse_board(&t).ok_or_else(|| format!("bad answer: {t}")));
            if let Err(e) = &r {
                warn!("hi scores: {e}");
            }
            *f = Fetch::Done(r);
        }
    }
}

fn refresh(
    pick: Res<Pick>,
    boards: Res<Boards>,
    best: Res<Best>,
    platform: Res<crate::Platform>,
    time: Res<Time>,
    mut lines: Query<(&mut Text, &mut TextColor, Option<&MenuLine>, Option<&RowLine>), Or<(With<MenuLine>, With<RowLine>)>>,
    mut notes: Query<
        (&mut Text, Has<Head>, Has<Status>),
        (Or<(With<Head>, With<Status>, With<Footer>)>, Without<MenuLine>, Without<RowLine>),
    >,
) {
    let board = match boards.0.get(&pick.board()) {
        Some(Fetch::Done(Ok(b))) => Some(b),
        _ => None,
    };
    let msg = match boards.0.get(&pick.board()) {
        _ if platform.fetch_board.is_none() => "the world board is not available in this build".to_string(),
        None | Some(Fetch::Waiting(_)) => format!("loading{}", ".".repeat(1 + (time.elapsed_secs() * 3.0) as usize % 3)),
        Some(Fetch::Done(Err(_))) => "can't reach the world board - A to retry".to_string(),
        Some(Fetch::Done(Ok(b))) if b.rows.is_empty() => "no runs yet - be the first!".to_string(),
        Some(Fetch::Done(Ok(_))) => String::new(),
    };
    for (mut t, mut color, menu, row) in &mut lines {
        if let Some(MenuLine(i)) = menu {
            let label = selector(&pick, *i);
            let on = pick.focus == *i;
            set_text(&mut t, format!("{}{label}", if on { "> " } else { "  " }));
            color.0 = if on { GOLD } else { INK };
        }
        if let Some(RowLine(i)) = row {
            let r = board.and_then(|b| b.rows.get(*i));
            set_text(&mut t, r.map_or(String::new(), |r| row_text(r, pick.city)));
            color.0 = match r {
                Some(r) if ours(r, &best) => GREEN,
                Some(r) => [GOLD, SILVER, BRONZE].get(r.rank as usize - 1).copied().unwrap_or(INK),
                None => INK,
            };
        }
    }
    let cols = if pick.city { ["TIME", "KILLS"] } else { ["SCORE", "STAGES"] };
    for (mut t, is_head, is_status) in &mut notes {
        let s = match (is_head, is_status) {
            (true, _) if board.is_some_and(|b| !b.rows.is_empty()) => columns("#", "NAME", cols[0], cols[1]),
            (true, _) => String::new(),
            (_, true) => msg.clone(),
            _ if pick.city => "fastest clear first".to_string(),
            _ => format!("your best on this machine  {}", best.hi),
        };
        set_text(&mut t, s);
    }
}

/// Selector row `i` as shown: name and `< value >`.
pub fn selector(pick: &Pick, i: usize) -> String {
    match i {
        0 => value("MODE", if pick.city { "CITY RUN" } else { "CLASSIC" }),
        1 => value("PLAYERS", if pick.players == 1 { "1 PLAYER" } else { "2 PLAYERS" }),
        2 if pick.city => value("DISTRICT", &format!("{}  {}", pick.district, crate::city_map::name(pick.district).to_uppercase())),
        _ => String::new(),
    }
}

/// Fixed widths, so the arrows of every selector line up and don't jump as the value changes.
fn value(name: &str, v: &str) -> String {
    format!("{name:<9} < {v:^16} >")
}

fn columns(rank: &str, names: &str, score: &str, progress: &str) -> String {
    format!(" {rank:>3}  {names:<17} {score:>7}  {progress:>6}")
}

/// One board line in fixed columns (the UI font is monospaced); City Run scores are clear times.
pub fn row_text(r: &Row, city: bool) -> String {
    let score = if city { scores::clock_text(r.score) } else { r.score.to_string() };
    columns(&r.rank.to_string(), &r.names, &score, &r.stages.to_string())
}

/// A row with a name this machine last entered (prefilled names count: they're what this couch sends).
pub fn ours(r: &Row, best: &Best) -> bool {
    r.names.split(" & ").any(|n| best.names.iter().any(|b| b == n))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selectors_walk_every_board_and_wrap() {
        let mut p = Pick::default();
        assert_eq!(p.board(), "1p");
        assert!(p.step(Dir::Down) && p.step(Dir::Right));
        assert_eq!(p.board(), "2p");
        p.step(Dir::Down);
        assert_eq!(p.focus, 0, "classic has two rows");
        p.step(Dir::Left);
        assert_eq!((p.board(), p.rows()), ("city1-2p".to_string(), 3));
        p.focus = 2;
        p.step(Dir::Left);
        assert_eq!(p.board(), format!("city{CITY_DISTRICTS}-2p"));
        p.step(Dir::Right);
        p.step(Dir::Right);
        assert_eq!(p.board(), "city2-2p");
        p.step(Dir::Down);
        assert_eq!(p.focus, 0);
        p.step(Dir::Right);
        assert_eq!((p.board(), p.focus), ("2p".to_string(), 0));
        p.focus = 1;
        p.step(Dir::Up);
        p.step(Dir::Up);
        assert_eq!(p.focus, 1);
    }

    #[test]
    fn rows_line_up_and_mark_this_couch() {
        let r = Row { rank: 1, score: 1503, stages: 31, names: "MAX & ZED".into() };
        assert_eq!(row_text(&r, true), "   1  MAX & ZED          2:30.3      31");
        assert_eq!(row_text(&r, false).len(), columns("#", "NAME", "SCORE", "STAGES").len());
        let best = Best { hi: 0, names: ["ZED".into(), "BBB".into()] };
        assert!(ours(&r, &best));
        assert!(!ours(&Row { names: "MAXI".into(), ..r }, &best));
        let district = selector(&Pick { city: true, district: 3, ..Pick::default() }, 2);
        assert!(
            district.starts_with("DISTRICT  < ") && district.contains(" 3  ") && district.len() == selector(&Pick::default(), 0).len(),
            "{district}"
        );
        assert!(selector(&Pick::default(), 2).is_empty());
    }
}
