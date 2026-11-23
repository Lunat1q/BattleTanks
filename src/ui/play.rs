//! PLAY from the title: one card per mode (CLASSIC, CITY RUN, VERSUS) with a picture, left/right picks, A goes on to
//! the mode's own picker (stage tiles, district map, VERSUS SETUP), B back to the title.

use super::*;
use crate::game::progress::Progress;
use crate::game::{Mode, Screen, Session, Sfx};
use crate::geom::Dir;
use crate::gfx::Gfx;
use crate::input::DeviceIntents;
use crate::rules::CITY_DISTRICTS;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Pick {
    Classic,
    City,
    Versus,
}

const PICKS: [Pick; 3] = [Pick::Classic, Pick::City, Pick::Versus];

impl Pick {
    fn name(self) -> &'static str {
        match self {
            Pick::Classic => "CLASSIC",
            Pick::City => "CITY RUN",
            Pick::Versus => "VERSUS",
        }
    }
    fn about(self) -> &'static str {
        match self {
            Pick::Classic => "30 stages - defend your base",
            Pick::City => "10 districts - beat the boss",
            Pick::Versus => "1 on 1 - destroy the other base",
        }
    }
    /// Can it be started with `players` joined? VERSUS needs both (on one couch or over LAN).
    fn ready(self, players: usize) -> bool {
        self != Pick::Versus || players == 2
    }
}

/// Card under the cursor; kept between visits.
#[derive(Resource, Default)]
struct Sel(usize);

pub(super) fn reset(world: &mut World) {
    world.insert_resource(Sel::default());
}

#[derive(Component)]
struct ModeCard(usize);
#[derive(Component)]
struct CardNote(usize);

const LOCKED: Color = Color::srgb(0.20, 0.22, 0.27);
/// Picture box, vh: 3:2.
const PIC: (f32, f32) = (36.0, 24.0);

pub fn plugin(app: &mut App) {
    app.init_resource::<Sel>()
        .add_systems(OnEnter(Screen::Play), spawn)
        .add_systems(Update, (input, refresh).chain().run_if(in_state(Screen::Play)));
}

/// The cursor after a left/right press, staying on the cards.
fn step(sel: usize, dir: Dir) -> usize {
    match dir {
        Dir::Left => sel.saturating_sub(1),
        Dir::Right => (sel + 1).min(PICKS.len() - 1),
        _ => sel,
    }
}

fn spawn(mut commands: Commands, gfx: Res<Gfx>, progress: Res<Progress>, settings: Res<crate::settings::Settings>, mut heads: Headings) {
    let stage = progress.open(Mode::Classic, settings.easy_mode).min(crate::levels::stage_count());
    commands.spawn((screen_root(), DespawnOnExit(Screen::Play))).with_children(|root| {
        root.spawn(heads.node("PLAY", Block::Brick, 7.5));
        root.spawn(Node { height: vh(2), ..default() });
        root.spawn(Node { column_gap: vh(4), ..default() }).with_children(|row| {
            for (i, pick) in PICKS.into_iter().enumerate() {
                row.spawn((
                    Node {
                        // Fixed, so the cards match whatever their text.
                        width: vh(50),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Center,
                        row_gap: vh(1.4),
                        padding: UiRect::all(vh(2)),
                        border: UiRect::all(vh(0.5)),
                        border_radius: BorderRadius::all(vh(1.6)),
                        ..default()
                    },
                    BorderColor::all(LOCKED),
                    BackgroundColor(Color::srgb(0.06, 0.065, 0.08)),
                    Outline::new(vh(0.5), vh(0.6), Color::NONE),
                    ModeCard(i),
                ))
                .with_children(|card| {
                    card.spawn((
                        Node {
                            width: vh(PIC.0),
                            height: vh(PIC.1),
                            align_items: AlignItems::Center,
                            justify_content: JustifyContent::Center,
                            column_gap: vh(3),
                            border_radius: BorderRadius::all(vh(1)),
                            overflow: Overflow::clip(),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.03, 0.03, 0.04)),
                    ))
                    .with_children(|pic| picture(pic, pick, &gfx, stage));
                    card.spawn(text(pick.name(), 4.6, INK));
                    card.spawn(text(pick.about(), 2.4, DIM));
                    card.spawn((text("", 2.6, GOLD), CardNote(i)));
                });
            }
        });
        root.spawn(Node { height: vh(2), ..default() });
        root.spawn(text("left/right choose   A play   B back", 2.4, DIM));
    });
}

/// What each card shows: the highest open classic stage's map, a slice of the City Run backdrop, two tanks face to face.
fn picture(pic: &mut ChildSpawnerCommands, pick: Pick, gfx: &Gfx, stage: usize) {
    match pick {
        Pick::Classic => {
            pic.spawn((ImageNode::new(gfx.stage_maps[stage - 1].clone()), Node { width: vh(PIC.1), height: vh(PIC.1), ..default() }));
        }
        Pick::City => {
            // The backdrop is 232 x 100; its middle 150 x 100 fills a 3:2 box.
            let (w, h) = (crate::gfx::art_map::W, crate::gfx::art_map::H);
            let x = (w - h * 1.5) / 2.0;
            pic.spawn((
                ImageNode { rect: Some(Rect::new(x, 0.0, w - x, h)), ..ImageNode::new(gfx.city_backdrop.clone()) },
                Node { width: percent(100), height: percent(100), ..default() },
            ));
        }
        Pick::Versus => {
            for (slot, color, angle) in [(0, GOLD, -90.0), (1, GREEN, 90.0)] {
                if slot == 1 {
                    pic.spawn(text("VS", 4.0, RED));
                }
                pic.spawn((
                    ImageNode::new(gfx.players[2][0].clone()).with_color(color),
                    Node { width: vh(10), height: vh(10), ..default() },
                    UiTransform::from_rotation(Rot2::degrees(angle)),
                ));
            }
        }
    }
}

fn input(
    devices: Res<DeviceIntents>,
    mut session: ResMut<Session>,
    mut sel: ResMut<Sel>,
    mut next: ResMut<NextState<Screen>>,
    mut sfx: MessageWriter<Sfx>,
) {
    let nav = devices.0.iter().map(|(_, i)| *i).fold(Default::default(), crate::input::merge);
    if let Some(dir) = nav.nav {
        let to = step(sel.0, dir);
        if to != sel.0 {
            sel.0 = to;
            sfx.write(Sfx::MenuMove);
        }
    }
    if nav.back {
        next.set(Screen::Title);
    } else if (nav.confirm || nav.start) && session.players() > 0 && PICKS[sel.0].ready(session.players()) {
        let (mode, screen) = match PICKS[sel.0] {
            Pick::Classic => (Mode::Classic, Screen::StageSelect),
            Pick::City => (Mode::City, Screen::CityMap),
            Pick::Versus => (Mode::Versus, Screen::VersusSetup),
        };
        session.mode = mode;
        sfx.write(Sfx::MenuPick);
        next.set(screen);
    }
}

fn refresh(
    time: Res<Time>,
    sel: Res<Sel>,
    session: Res<Session>,
    progress: Res<Progress>,
    settings: Res<crate::settings::Settings>,
    mut cards: Query<(&ModeCard, &mut BorderColor, &mut Outline)>,
    mut notes: Query<(&CardNote, &mut Text, &mut TextColor)>,
) {
    let pulse = 0.75 + 0.25 * (time.elapsed_secs() * 5.0).sin();
    for (card, mut border, mut outline) in &mut cards {
        let pick = PICKS[card.0];
        *border = BorderColor::all(if pick.ready(session.players()) { INK } else { LOCKED });
        outline.color = if card.0 == sel.0 { Color::WHITE.with_alpha(pulse) } else { Color::NONE };
    }
    for (note, mut t, mut color) in &mut notes {
        let (s, c) = match PICKS[note.0] {
            Pick::Classic => (opened("STAGE", progress.open(Mode::Classic, settings.easy_mode), crate::levels::stage_count()), GOLD),
            Pick::City => (opened("DISTRICT", progress.open(Mode::City, settings.easy_mode), CITY_DISTRICTS), GOLD),
            Pick::Versus if session.players() < 2 => ("NEEDS 2 PLAYERS".into(), DIM),
            Pick::Versus => (format!("{} MAPS", crate::versus_map::count()), GOLD),
        };
        set_text(&mut t, s);
        color.0 = c;
    }
}

/// How far the players have got: the highest open one, or all cleared.
fn opened(what: &str, open: usize, count: usize) -> String {
    if open > count { "ALL CLEARED".into() } else { format!("UP TO {what} {open}") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cursor_stays_on_the_cards_and_notes_read_right() {
        assert_eq!(step(0, Dir::Left), 0);
        assert_eq!(step(0, Dir::Right), 1);
        assert_eq!(step(2, Dir::Right), 2);
        assert_eq!(step(1, Dir::Up), 1);
        assert_eq!(opened("STAGE", 1, 30), "UP TO STAGE 1");
        assert_eq!(opened("DISTRICT", 11, 10), "ALL CLEARED");
        assert!(Pick::Classic.ready(1) && Pick::City.ready(1));
        assert!(Pick::Versus.ready(2) && !Pick::Versus.ready(1));
    }
}
