//! Stage banner, pause menu, results tally and game over.

use super::*;
use crate::game::{Paused, Phase, Screen, Session, Sfx, StageRun};
use crate::geom::Dir;
use crate::input::{DeviceIntents, Intent};
use crate::rules::EnemyKind;

#[derive(Component)]
struct Banner;
#[derive(Component)]
struct PauseOverlay;
#[derive(Component)]
struct PauseNote;

/// Seconds the current results/game-over screen has been up.
#[derive(Resource, Default)]
struct ScreenTime(f32);

#[derive(Resource, Default)]
struct PauseSel(usize);

const PAUSE_ITEMS: [&str; 3] = ["RESUME", "QUIT TO TITLE", "EXIT GAME"];
const KIND_NAMES: [&str; 4] = ["LIGHT ", "SCOUT ", "GUNNER", "HEAVY "];

pub fn plugin(app: &mut App) {
    app.init_resource::<ScreenTime>()
        .init_resource::<PauseSel>()
        .add_systems(OnEnter(Screen::Stage), spawn_overlays)
        .add_systems(Update, (banner, pause_menu).run_if(in_state(Screen::Stage)))
        .add_systems(OnEnter(Screen::Results), spawn_results)
        .add_systems(OnEnter(Screen::GameOver), spawn_game_over)
        .add_systems(Update, results_input.run_if(in_state(Screen::Results)))
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
    let (s, c, back) = match run.phase {
        Phase::Intro => (format!("STAGE {}", session.stage), INK, 0.75),
        Phase::Play => (String::new(), INK, 0.0),
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
) {
    if let Ok(mut v) = overlay.single_mut() {
        v.set_if_neq(if paused.0 { Visibility::Inherited } else { Visibility::Hidden });
    }
    let lost: Vec<usize> = (0..2).filter(|&i| session.slots[i].as_ref().is_some_and(|s| !devices.connected(s.device))).collect();
    for (mut t, line, note) in &mut lines {
        if let Some(MenuLine(i)) = line {
            set_text(&mut t, menu_label(sel.0 == *i, PAUSE_ITEMS[*i]));
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
    if input.back && lost.is_empty() {
        paused.0 = false;
    }
    if input.confirm {
        sfx.write(Sfx::MenuPick);
        match sel.0 {
            0 if lost.is_empty() => paused.0 = false,
            1 => next.set(Screen::Title),
            2 => {
                exit.write(AppExit::Success);
            }
            _ => {}
        }
    }
}

fn table(session: &Session) -> String {
    let present: Vec<usize> = (0..2).filter(|&i| session.slots[i].is_some()).collect();
    let mut out = format!("{:8}", "");
    for &i in &present {
        out += &format!("{:>16}", format!("{}P", i + 1));
    }
    for k in EnemyKind::ALL {
        out += &format!("\n{:8}", KIND_NAMES[k.index()]);
        for &i in &present {
            let n = session.slots[i].as_ref().map_or(0, |s| s.kills[k.index()]);
            out += &format!("{:>16}", format!("{n:>2} x {:>3}", k.points()));
        }
    }
    out += &format!("\n\n{:8}", "SCORE");
    for &i in &present {
        out += &format!("{:>16}", session.slots[i].as_ref().map_or(0, |s| s.score));
    }
    out
}

fn spawn_results(mut commands: Commands, session: Res<Session>, mut time: ResMut<ScreenTime>) {
    time.0 = 0.0;
    commands.spawn((screen_root(), DespawnOnExit(Screen::Results))).with_children(|r| {
        r.spawn(text(format!("STAGE {} CLEAR", session.stage), 8.0, GOLD));
        r.spawn(text(table(&session), 3.8, INK));
        r.spawn(text("press A to continue", 3.0, DIM));
    });
}

fn spawn_game_over(mut commands: Commands, session: Res<Session>, mut time: ResMut<ScreenTime>) {
    time.0 = 0.0;
    commands.spawn((screen_root(), DespawnOnExit(Screen::GameOver))).with_children(|r| {
        r.spawn(text("GAME OVER", 12.0, RED));
        r.spawn(text(format!("reached stage {}", session.stage), 3.6, DIM));
        r.spawn(text(table(&session), 3.8, INK));
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
    t: Res<Time>,
    mut time: ResMut<ScreenTime>,
    mut session: ResMut<Session>,
    devices: Res<DeviceIntents>,
    mut next: ResMut<NextState<Screen>>,
) {
    if advance_pressed(&mut time, t.delta_secs(), &session, &devices) || time.0 > 10.0 {
        session.stage += 1;
        next.set(Screen::Stage);
    }
}

fn game_over_input(
    t: Res<Time>,
    mut time: ResMut<ScreenTime>,
    session: Res<Session>,
    devices: Res<DeviceIntents>,
    mut next: ResMut<NextState<Screen>>,
) {
    if advance_pressed(&mut time, t.delta_secs(), &session, &devices) {
        next.set(Screen::Title);
    }
}
