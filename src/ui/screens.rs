//! Stage banner, pause menu, results and game over screens (the tally itself is `tally.rs`).

use super::tally::{RESULTS_WAIT, Tally, kill_maxes, table, tally_len, tally_view};
use super::*;
use crate::achieve::Record;
use crate::game::versus::Match;
use crate::game::{Mode, Paused, Phase, Screen, Session, Sfx, Sim, StageFlow, StageRun};
use crate::geom::Dir;
use crate::input::{DeviceIntents, Intent};

/// The stage banner's block heading; holds the text it shows.
#[derive(Component, Default)]
struct Banner(String);
/// The banner's backing plate; shown with the banner.
#[derive(Component)]
struct BannerPlate;
/// The district's name under its banner.
#[derive(Component)]
struct BannerSub;
#[derive(Component)]
struct PauseOverlay;
#[derive(Component)]
struct PauseNote;

/// Seconds the current results/game-over screen has been up.
#[derive(Resource, Default)]
pub struct ScreenTime(pub f32);

#[derive(Resource, Default)]
struct PauseSel(usize);

const PAUSE_ITEMS: [&str; 3] = ["RESUME", "QUIT TO TITLE", "EXIT GAME"];

pub fn plugin(app: &mut App) {
    app.init_resource::<ScreenTime>()
        .init_resource::<PauseSel>()
        .add_systems(OnEnter(Screen::Stage), spawn_overlays)
        .add_systems(Update, (banner, pause_menu.after(StageFlow).before(Sim)).run_if(in_state(Screen::Stage)))
        .add_systems(OnEnter(Screen::Results), spawn_results)
        .add_systems(OnEnter(Screen::GameOver), spawn_game_over)
        .add_systems(Update, (super::tally::tally, results_input).chain().run_if(in_state(Screen::Results)))
        .add_systems(Update, game_over_input.run_if(in_state(Screen::GameOver)));
}

fn overlay(bg: f32) -> impl Bundle {
    (
        Node { position_type: PositionType::Absolute, ..screen_root() },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, bg)),
        DespawnOnExit(Screen::Stage),
    )
}

/// Banner heading for `phase`; empty while playing. Classic is brick, City Run steel, VERSUS steel then the
/// round's winner in gold.
fn banner_head(phase: Phase, session: &Session, duel: &Match) -> (String, Block) {
    if session.mode == Mode::Versus {
        return match phase {
            Phase::Intro => (format!("ROUND {}", duel.round), Block::Steel),
            Phase::Play => (String::new(), Block::Steel),
            Phase::Won | Phase::Lost => (super::versus::head(duel.last), Block::Gold),
        };
    }
    let m = if session.mode == Mode::City { Block::Steel } else { Block::Brick };
    let s = match phase {
        Phase::Intro => format!("{} {}", place(session), session.stage),
        Phase::Play => String::new(),
        Phase::Won => format!("{} CLEAR", place(session)),
        Phase::Lost => "GAME OVER".into(),
    };
    (s, if phase == Phase::Lost { Block::Brick } else { m })
}

/// Dark plate with a steel rim behind the stage banner: a brick heading alone got lost over a busy field (owner).
fn banner_plate() -> impl Bundle {
    (
        Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::axes(vh(4.0), vh(2.4)),
            border: UiRect::all(vh(0.8)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.03, 0.03, 0.05, 0.92)),
        BorderColor::all(Color::srgb(0.66, 0.70, 0.78)),
        Outline::new(vh(0.4), Val::ZERO, Color::srgb(0.24, 0.26, 0.31)),
    )
}

fn spawn_overlays(mut commands: Commands, mut sel: ResMut<PauseSel>, session: Res<Session>, duel: Res<Match>, mut heads: Headings) {
    sel.0 = 0;
    // Painted now, so the stage's end doesn't stall a frame to paint its banner.
    if session.mode == Mode::Versus {
        for last in [Some(0), Some(1), None] {
            heads.image(&super::versus::head(last), Block::Gold);
        }
    }
    for phase in [Phase::Won, Phase::Lost] {
        let (s, m) = banner_head(phase, &session, &duel);
        heads.image(&s, m);
    }
    commands.spawn(overlay(0.0)).with_children(|b| {
        b.spawn((banner_plate(), Visibility::Hidden, BannerPlate)).with_children(|p| {
            p.spawn((ImageNode::default(), Node { height: vh(9.0), ..default() }, Banner::default()));
            p.spawn((text("", 4.6, INK), BannerSub));
        });
    });
    commands.spawn((overlay(0.65), PauseOverlay, Visibility::Hidden)).with_children(|p| {
        p.spawn(heads.node("PAUSED", Block::Ice, 8.0));
        for i in 0..PAUSE_ITEMS.len() {
            p.spawn((text("", 4.6, INK), MenuLine(i)));
        }
        p.spawn((text("", 3.4, RED), PauseNote));
    });
}

fn banner(
    run: Option<Res<StageRun>>,
    session: Res<Session>,
    duel: Res<Match>,
    mut heads: Headings,
    mut q: Query<(&mut Banner, &mut ImageNode)>,
    mut plate: Query<(&mut Visibility, &ChildOf), With<BannerPlate>>,
    mut sub: Query<&mut Text, With<BannerSub>>,
    mut bg: Query<&mut BackgroundColor>,
) {
    let Some(run) = run else { return };
    let Ok((mut shown, mut img)) = q.single_mut() else { return };
    let Ok((mut vis, parent)) = plate.single_mut() else { return };
    let (s, m) = banner_head(run.phase, &session, &duel);
    if shown.0 != s {
        if !s.is_empty() {
            img.image = heads.image(&s, m);
        }
        vis.set_if_neq(if s.is_empty() { Visibility::Hidden } else { Visibility::Inherited });
        shown.0 = s;
    }
    let city = session.mode == Mode::City;
    if let Ok(mut t) = sub.single_mut() {
        let name = if city && run.phase == Phase::Intro { crate::city_map::name(session.stage).to_uppercase() } else { String::new() };
        set_text(&mut t, name);
    }
    // Dark enough that a brick heading reads over a brick field.
    let back = match run.phase {
        Phase::Intro => 0.85,
        Phase::Play => 0.0,
        _ => 0.55,
    };
    if let Ok(mut b) = bg.get_mut(parent.parent()) {
        b.0 = Color::srgba(0.0, 0.0, 0.0, back);
    }
}

fn pause_menu(
    mut paused: ResMut<Paused>,
    session: Res<Session>,
    devices: Res<DeviceIntents>,
    mut sel: ResMut<PauseSel>,
    mut overlay: Query<&mut Visibility, With<PauseOverlay>>,
    mut lines: Query<(&mut Text, Option<&MenuLine>, Has<PauseNote>)>,
    mut next: ResMut<NextState<Screen>>,
    mut exit: MessageWriter<AppExit>,
    mut sfx: MessageWriter<Sfx>,
    mut net: ResMut<crate::net::Net>,
) {
    if let Ok(mut v) = overlay.single_mut() {
        v.set_if_neq(if paused.0 { Visibility::Inherited } else { Visibility::Hidden });
    }
    let lost: Vec<usize> = (0..2).filter(|&i| session.slots[i].as_ref().is_some_and(|s| !devices.connected(s.device))).collect();
    for (mut t, line, note) in &mut lines {
        if let Some(MenuLine(i)) = line {
            // Linked, the last item ends the session for both, rather than closing one app under the other player.
            let label = if *i == 2 && net.linked() { "LEAVE LAN" } else { PAUSE_ITEMS[*i] };
            set_text(&mut t, menu_label(sel.0 == *i, label));
        }
        if note {
            let msg =
                lost.iter().map(|i| format!("{}P controller disconnected - press A on a controller", i + 1)).collect::<Vec<_>>().join("\n");
            set_text(&mut t, msg);
        }
    }
    if !paused.0 {
        sel.0 = 0;
        return;
    }
    let input: Intent = (0..2).map(|i| session.intent(i, &devices)).fold(Intent::default(), crate::input::merge);
    match input.nav {
        Some(Dir::Up) => sel.0 = (sel.0 + PAUSE_ITEMS.len() - 1) % PAUSE_ITEMS.len(),
        Some(Dir::Down) => sel.0 = (sel.0 + 1) % PAUSE_ITEMS.len(),
        _ => {}
    }
    if input.nav.is_some() {
        sfx.write(Sfx::MenuMove);
    }
    if input.back && !input.pause && lost.is_empty() {
        paused.0 = false;
    }
    if input.confirm {
        sfx.write(Sfx::MenuPick);
        match sel.0 {
            0 if lost.is_empty() => paused.0 = false,
            1 => next.set(Screen::Title),
            2 if net.linked() => {
                net.leave = true;
                next.set(Screen::Title);
            }
            2 => {
                exit.write(AppExit::Success);
            }
            _ => {}
        }
    }
}

fn place(session: &Session) -> &'static str {
    if session.mode == Mode::City { "DISTRICT" } else { "STAGE" }
}

fn spawn_results(mut commands: Commands, session: Res<Session>, mut time: ResMut<ScreenTime>, mut heads: Headings) {
    time.0 = 0.0;
    let (head, m) = banner_head(Phase::Won, &session, &Match::default());
    let head = head.replace(" CLEAR", &format!(" {} CLEAR", session.stage));
    commands.spawn((screen_root(), DespawnOnExit(Screen::Results))).with_children(|r| {
        r.spawn(heads.node(&head, m, 7.0));
        r.spawn((text(table(&session, Some(tally_view(0.0, kill_maxes(&session)))), 3.8, INK), Tally));
        r.spawn(text("press A to continue", 3.0, DIM));
    });
}

fn spawn_game_over(mut commands: Commands, session: Res<Session>, mut time: ResMut<ScreenTime>, mut heads: Headings) {
    time.0 = 0.0;
    commands.spawn((screen_root(), DespawnOnExit(Screen::GameOver))).with_children(|r| {
        r.spawn(heads.node("GAME OVER", Block::Brick, 10.0));
        r.spawn(text(format!("reached {} {}", place(&session).to_lowercase(), session.stage), 3.6, DIM));
        r.spawn(text(table(&session, None), 3.8, INK));
        r.spawn(text(format!("HI SCORE  {}", session.hi_score), 4.2, GOLD));
        r.spawn(text("press A", 3.0, DIM));
    });
}

/// Any joined device pressed A/Start, after a short guard so a held fire button doesn't skip the screen.
pub(super) fn advance_pressed(time: &mut ScreenTime, dt: f32, session: &Session, devices: &DeviceIntents) -> bool {
    time.0 += dt;
    time.0 > 1.0
        && (0..2).any(|i| {
            let it = session.intent(i, devices);
            it.confirm || it.start
        })
}

fn results_input(
    time: Res<ScreenTime>,
    mut session: ResMut<Session>,
    devices: Res<DeviceIntents>,
    platform: Res<crate::Platform>,
    net: Res<crate::net::Net>,
    rec: Res<Record>,
    settings: Res<crate::settings::Settings>,
    mut next: ResMut<NextState<Screen>>,
) {
    // After the count-up (or the press that skipped it): a 1 s guard against a held button, then A or the wait moves on.
    let done = time.0 - tally_len(kill_maxes(&session));
    let pressed = (0..2).any(|i| {
        let it = session.intent(i, &devices);
        it.confirm || it.start
    });
    if (done > 1.0 && pressed) || done > RESULTS_WAIT {
        if session.mode == Mode::City {
            // The district's new achievements and world board, then the map (pick the next district or replay one).
            next.set(super::unlocks::after_play(&rec, &platform, &session, net.linked(), !settings.ranked(session.players())));
        } else {
            session.stage += 1;
            next.set(Screen::Stage);
        }
    }
}

fn game_over_input(
    t: Res<Time>,
    mut time: ResMut<ScreenTime>,
    session: Res<Session>,
    devices: Res<DeviceIntents>,
    platform: Res<crate::Platform>,
    net: Res<crate::net::Net>,
    rec: Res<Record>,
    settings: Res<crate::settings::Settings>,
    mut next: ResMut<NextState<Screen>>,
) {
    if advance_pressed(&mut time, t.delta_secs(), &session, &devices) {
        // The run's new achievements and world board, then the title; City Run: back to the district map to retry.
        next.set(super::unlocks::after_play(&rec, &platform, &session, net.linked(), !settings.ranked(session.players())));
    }
}
