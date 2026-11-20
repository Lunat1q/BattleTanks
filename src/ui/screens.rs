//! Stage banner, pause menu, results and game over screens (the tally itself is `tally.rs`).

use super::tally::{RESULTS_WAIT, Tally, kill_maxes, table, tally_len, tally_view};
use super::*;
use crate::game::{Mode, Paused, Phase, Screen, Session, Sfx, StageRun};
use crate::geom::Dir;
use crate::input::{DeviceIntents, Intent};

#[derive(Component)]
struct Banner;
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
        .add_systems(Update, (banner, pause_menu).run_if(in_state(Screen::Stage)))
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

fn spawn_overlays(mut commands: Commands, mut sel: ResMut<PauseSel>) {
    sel.0 = 0;
    commands.spawn(overlay(0.0)).with_child((text("", 9.0, INK), Banner));
    commands.spawn((overlay(0.65), PauseOverlay, Visibility::Hidden)).with_children(|p| {
        p.spawn(text("PAUSED", 9.0, GOLD));
        for i in 0..PAUSE_ITEMS.len() {
            p.spawn((text("", 4.6, INK), MenuLine(i)));
        }
        p.spawn((text("", 3.4, RED), PauseNote));
    });
}

fn banner(
    run: Option<Res<StageRun>>,
    session: Res<Session>,
    mut q: Query<(&mut Text, &mut TextColor, &ChildOf), With<Banner>>,
    mut bg: Query<&mut BackgroundColor>,
) {
    let Some(run) = run else { return };
    let Ok((mut t, mut color, parent)) = q.single_mut() else { return };
    let city = session.mode == Mode::City;
    let (s, c, back) = match run.phase {
        Phase::Intro if city => (
            format!(
                "DISTRICT {}
{}",
                session.stage,
                crate::city_map::name(session.stage).to_uppercase()
            ),
            INK,
            0.75,
        ),
        Phase::Intro => (format!("STAGE {}", session.stage), INK, 0.75),
        Phase::Play => (String::new(), INK, 0.0),
        Phase::Won if city => ("DISTRICT CLEAR".into(), GOLD, 0.0),
        Phase::Won => ("STAGE CLEAR".into(), GOLD, 0.0),
        Phase::Lost => ("GAME OVER".into(), RED, 0.35),
    };
    set_text(&mut t, s);
    color.0 = c;
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

fn spawn_results(mut commands: Commands, session: Res<Session>, mut time: ResMut<ScreenTime>) {
    time.0 = 0.0;
    commands.spawn((screen_root(), DespawnOnExit(Screen::Results))).with_children(|r| {
        r.spawn(text(format!("{} {} CLEAR", place(&session), session.stage), 8.0, GOLD));
        r.spawn((text(table(&session, Some(tally_view(0.0, kill_maxes(&session)))), 3.8, INK), Tally));
        r.spawn(text("press A to continue", 3.0, DIM));
    });
}

fn spawn_game_over(mut commands: Commands, session: Res<Session>, mut time: ResMut<ScreenTime>) {
    time.0 = 0.0;
    commands.spawn((screen_root(), DespawnOnExit(Screen::GameOver))).with_children(|r| {
        r.spawn(text("GAME OVER", 12.0, RED));
        r.spawn(text(format!("reached {} {}", place(&session).to_lowercase(), session.stage), 3.6, DIM));
        r.spawn(text(table(&session, None), 3.8, INK));
        r.spawn(text(format!("HI SCORE  {}", session.hi_score), 4.2, GOLD));
        r.spawn(text("press A", 3.0, DIM));
    });
}

/// Any joined device pressed A/Start, after a short guard so a held fire button doesn't skip the screen.
fn advance_pressed(time: &mut ScreenTime, dt: f32, session: &Session, devices: &DeviceIntents) -> bool {
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
            // The clear's world board, then the district map (pick the next district or replay one).
            // LAN runs skip the world board: name entry waits on a network answer, which the two machines
            // would get at different ticks.
            next.set(if super::board::wants_board(&session, &platform) && !net.linked() { Screen::Board } else { Screen::CityMap });
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
    mut next: ResMut<NextState<Screen>>,
) {
    if advance_pressed(&mut time, t.delta_secs(), &session, &devices) {
        // The run's world board, then the title; City Run: back to the district map to retry.
        next.set(if super::board::wants_board(&session, &platform) && !net.linked() {
            Screen::Board
        } else {
            super::board::onward(&session)
        });
    }
}
