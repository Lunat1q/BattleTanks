//! VERSUS army draft (ADR 0019): one card per player, each worked with that player's own pad. Up/down picks a
//! tank kind or AUTO, left/right changes the count within the points, A on AUTO fills a mixed army, A elsewhere
//! is ready (B takes it back). Cards open on the match's last armies (AUTO for a new match). Both ready: the duel
//! starts. B on a card that isn't ready goes back to VERSUS SETUP.

use super::*;
use crate::game::versus::Match;
use crate::game::{Screen, Session, Sfx};
use crate::geom::Dir;
use crate::gfx::Gfx;
use crate::input::DeviceIntents;
use crate::rules::{self, EnemyKind};

/// The AUTO row, under the four kinds.
const AUTO: usize = 4;

#[derive(Clone, Copy, Default, PartialEq, Debug)]
struct Draft {
    counts: [u32; 4],
    row: usize,
    ready: bool,
}

impl Draft {
    fn of(army: &[EnemyKind]) -> Self {
        let mut d = Draft::default();
        for k in army {
            d.counts[k.index()] += 1;
        }
        d
    }

    fn left(&self, budget: u32) -> u32 {
        let spent: u32 = EnemyKind::ALL.iter().map(|k| k.cost() * self.counts[k.index()]).sum();
        budget.saturating_sub(spent)
    }

    /// Up/down moves between the rows, left/right takes one off or adds one if the points allow. True if anything
    /// changed.
    fn nav(&mut self, dir: Dir, budget: u32) -> bool {
        let before = *self;
        match dir {
            Dir::Up => self.row = self.row.saturating_sub(1),
            Dir::Down => self.row = (self.row + 1).min(AUTO),
            Dir::Left if self.row < AUTO => self.counts[self.row] = self.counts[self.row].saturating_sub(1),
            Dir::Right if self.row < AUTO && EnemyKind::ALL[self.row].cost() <= self.left(budget) => self.counts[self.row] += 1,
            _ => {}
        }
        *self != before
    }

    /// A: on AUTO the mixed army, elsewhere ready.
    fn confirm(&mut self, budget: u32) {
        if self.row == AUTO {
            self.counts = Draft::of(&rules::auto_army(budget)).counts;
        } else {
            self.ready = true;
        }
    }

    fn roster(&self) -> Vec<EnemyKind> {
        EnemyKind::ALL.iter().flat_map(|&k| std::iter::repeat_n(k, self.counts[k.index()] as usize)).collect()
    }

    fn line(&self, row: usize) -> String {
        let s = match row {
            AUTO => "AUTO MIX".into(),
            r => {
                let k = EnemyKind::ALL[r];
                let n = self.counts[r];
                let n = if r == self.row && !self.ready { format!("< {n} >") } else { n.to_string() };
                format!("{}  {} PT   {n}", k.name(), k.cost())
            }
        };
        if row == AUTO { menu_label(row == self.row && !self.ready, &s) } else { s }
    }
}

#[derive(Resource, Default)]
struct Drafts([Draft; 2]);

#[derive(Component)]
struct Line(usize, usize);
#[derive(Component)]
struct Status(usize);

pub fn plugin(app: &mut App) {
    app.init_resource::<Drafts>()
        .add_systems(OnEnter(Screen::Draft), spawn)
        .add_systems(Update, (input, refresh).chain().run_if(in_state(Screen::Draft)));
}

fn spawn(mut commands: Commands, gfx: Res<Gfx>, duel: Res<Match>, mut drafts: ResMut<Drafts>, mut heads: Headings) {
    drafts.0 = [0, 1].map(|p| Draft::of(&duel.armies[p]));
    commands.spawn((screen_root(), DespawnOnExit(Screen::Draft))).with_children(|root| {
        root.spawn(heads.node("DRAFT", Block::Steel, 7.0));
        root.spawn(text(format!("{} points each - NPCs join the fight one by one", duel.budget), 2.6, DIM));
        root.spawn(Node { height: vh(1), ..default() });
        root.spawn(Node { column_gap: vh(6), ..default() }).with_children(|row| {
            for p in 0..2 {
                let color = crate::gfx::player_color(p);
                row.spawn((
                    Node {
                        width: vh(56),
                        flex_direction: FlexDirection::Column,
                        row_gap: vh(1.2),
                        padding: UiRect::all(vh(2.4)),
                        border: UiRect::all(vh(0.5)),
                        border_radius: BorderRadius::all(vh(1.6)),
                        ..default()
                    },
                    BorderColor::all(color),
                    BackgroundColor(Color::srgb(0.06, 0.065, 0.08)),
                ))
                .with_children(|card| {
                    card.spawn((text(format!("PLAYER {}", p + 1), 4.4, color), Node { align_self: AlignSelf::Center, ..default() }));
                    for r in 0..=AUTO {
                        card.spawn(Node { column_gap: vh(2), align_items: AlignItems::Center, height: vh(5), ..default() }).with_children(
                            |line| {
                                if r < AUTO {
                                    line.spawn((
                                        ImageNode::new(gfx.enemies[r][0].clone()).with_color(crate::gfx::npc_color(p)),
                                        Node { width: vh(4.5), height: vh(4.5), ..default() },
                                    ));
                                }
                                line.spawn((text("", 3.2, DIM), Line(p, r)));
                            },
                        );
                    }
                    card.spawn((text("", 3.2, GOLD), Status(p), Node { align_self: AlignSelf::Center, ..default() }));
                });
            }
        });
        root.spawn(Node { height: vh(2), ..default() });
        root.spawn(text("each on their own pad:   up/down choose   left/right count   A ready   B back", 2.4, DIM));
    });
}

fn input(
    session: Res<Session>,
    devices: Res<DeviceIntents>,
    mut duel: ResMut<Match>,
    mut drafts: ResMut<Drafts>,
    mut next: ResMut<NextState<Screen>>,
    mut sfx: MessageWriter<Sfx>,
) {
    let budget = duel.budget;
    for p in 0..2 {
        let it = session.intent(p, &devices);
        let d = &mut drafts.0[p];
        if it.back {
            if !d.ready {
                next.set(Screen::VersusSetup);
                return;
            }
            d.ready = false;
            sfx.write(Sfx::MenuMove);
        } else if d.ready {
            continue;
        } else if it.confirm || it.start {
            d.confirm(budget);
            sfx.write(Sfx::MenuPick);
        } else if it.nav.is_some_and(|dir| d.nav(dir, budget)) {
            sfx.write(Sfx::MenuMove);
        }
    }
    if drafts.0.iter().all(|d| d.ready) {
        duel.armies = [0, 1].map(|p| drafts.0[p].roster());
        next.set(Screen::Stage);
    }
}

fn refresh(
    duel: Res<Match>,
    drafts: Res<Drafts>,
    mut lines: Query<(&Line, &mut Text, &mut TextColor), Without<Status>>,
    mut status: Query<(&Status, &mut Text)>,
) {
    for (line, mut t, mut color) in &mut lines {
        let d = &drafts.0[line.0];
        set_text(&mut t, d.line(line.1));
        color.0 = if line.1 == d.row && !d.ready { INK } else { DIM };
    }
    for (s, mut t) in &mut status {
        let d = &drafts.0[s.0];
        let left = d.left(duel.budget);
        set_text(&mut t, if d.ready { "READY   B change".into() } else { format!("{left} POINTS LEFT   A ready") });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_draft_stays_within_its_points() {
        let mut d = Draft::of(&rules::auto_army(20));
        assert_eq!((d.counts, d.left(20)), ([3, 3, 2, 1], 0));
        assert!(!d.nav(Dir::Right, 20), "no points left");
        d.nav(Dir::Left, 20);
        assert_eq!((d.counts[0], d.left(20)), (2, 1));
        d.nav(Dir::Down, 20);
        d.nav(Dir::Down, 20);
        d.nav(Dir::Down, 20);
        assert!(!d.nav(Dir::Right, 20), "a HEAVY costs 5, 1 left");
        d.nav(Dir::Left, 20);
        assert_eq!(d.left(20), 6);
        assert!(d.nav(Dir::Right, 20));
        for _ in 0..9 {
            d.nav(Dir::Left, 20);
        }
        assert_eq!(d.counts[3], 0, "never below none");
        d.nav(Dir::Down, 20);
        assert!(!d.nav(Dir::Down, 20) && !d.nav(Dir::Right, 20), "AUTO is the last row and has no count");
        d.confirm(20);
        assert_eq!((d.counts, d.ready), ([3, 3, 2, 1], false), "AUTO refills, not ready yet");
        d.nav(Dir::Up, 20);
        d.confirm(20);
        assert!(d.ready);
        let mut r = d.roster();
        r.sort_by_key(|k| k.index());
        let mut auto = rules::auto_army(20);
        auto.sort_by_key(|k| k.index());
        assert_eq!(r, auto);
        assert_eq!(Draft::default().roster(), [], "an empty army is allowed");
    }
}
