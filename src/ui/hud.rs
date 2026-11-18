//! In-stage HUD: side panels left and right of the field (lives, score, star level, enemy reserve, stage).

use super::*;
use crate::game::{Screen, Session, StageRun};
use crate::gfx::{Gfx, VIEW_HEIGHT, player_color};

#[derive(Component, Clone, Copy, PartialEq)]
enum Hud {
    Lives(usize),
    Score(usize),
    Level(usize),
    Stage,
    Hi,
}

#[derive(Component)]
struct EnemyIcon(usize);

/// Field width as % of screen height (the camera shows `VIEW_HEIGHT` units vertically).
const FIELD_VH: f32 = crate::geom::FIELD / VIEW_HEIGHT * 100.0;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Stage), spawn).add_systems(Update, refresh.run_if(in_state(Screen::Stage)));
}

fn player_block(p: &mut ChildSpawnerCommands, i: usize, gfx: &Gfx, session: &Session) {
    if session.slots[i].is_none() {
        return;
    }
    let color = player_color(i);
    p.spawn(Node { flex_direction: FlexDirection::Column, row_gap: vh(0.8), align_items: AlignItems::Center, ..default() }).with_children(
        |b| {
            b.spawn(text(format!("{}P", i + 1), 6.0, color));
            b.spawn(Node { align_items: AlignItems::Center, column_gap: vh(1), ..default() }).with_children(|row| {
                row.spawn((
                    ImageNode::new(gfx.players[0][0].clone()).with_color(color),
                    Node { width: vh(4.5), height: vh(4.5), ..default() },
                ));
                row.spawn((text("", 4.5, INK), Hud::Lives(i)));
            });
            b.spawn((text("", 3.6, INK), Hud::Score(i)));
            b.spawn((text("", 2.8, DIM), Hud::Level(i)));
        },
    );
}

fn spawn(mut commands: Commands, gfx: Res<Gfx>, session: Res<Session>) {
    let panel = |justify| Node {
        flex_grow: 1.0,
        height: percent(100),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: justify,
        padding: UiRect::vertical(vh(6)),
        ..default()
    };
    commands.spawn((Node { width: percent(100), height: percent(100), ..default() }, DespawnOnExit(Screen::Stage))).with_children(|root| {
        root.spawn(panel(JustifyContent::SpaceBetween)).with_children(|left| {
            player_block(left, 0, &gfx, &session);
            left.spawn((text("", 2.6, DIM), Hud::Hi));
            left.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() }).with_children(|s| {
                s.spawn((ImageNode::new(gfx.flag.clone()), Node { width: vh(7), height: vh(7), ..default() }));
                s.spawn((text("", 5.0, INK), Hud::Stage));
            });
        });
        root.spawn(Node { width: vh(FIELD_VH + 2.0), height: percent(100), ..default() });
        root.spawn(panel(JustifyContent::SpaceBetween)).with_children(|right| {
            right
                .spawn(Node { width: vh(8.4), flex_wrap: FlexWrap::Wrap, row_gap: vh(0.6), column_gap: vh(0.6), ..default() })
                .with_children(|grid| {
                    for i in 0..crate::rules::ENEMIES_PER_STAGE {
                        grid.spawn((
                            ImageNode::new(gfx.enemies[0][0].clone()).with_color(Color::srgb(0.62, 0.65, 0.72)),
                            Node { width: vh(3.6), height: vh(3.6), ..default() },
                            EnemyIcon(i),
                        ));
                    }
                });
            player_block(right, 1, &gfx, &session);
        });
    });
}

fn refresh(
    session: Res<Session>,
    run: Option<Res<StageRun>>,
    mut texts: Query<(&mut Text, &Hud)>,
    mut icons: Query<(&mut Visibility, &EnemyIcon)>,
) {
    let Some(run) = run else { return };
    for (mut t, hud) in &mut texts {
        let s = match *hud {
            Hud::Lives(i) => session.slots[i].as_ref().map(|s| format!("x {}", s.lives)),
            Hud::Score(i) => session.slots[i].as_ref().map(|s| format!("{:>7}", s.score)),
            Hud::Level(i) => session.slots[i].as_ref().map(|s| if s.out { "OUT".into() } else { format!("POWER {}", s.level) }),
            Hud::Stage => Some(session.stage.to_string()),
            Hud::Hi => Some(format!("HI {}", session.hi_score)),
        };
        set_text(&mut t, s.unwrap_or_default());
    }
    let reserve = run.roster.len().saturating_sub(run.next_enemy);
    for (mut vis, icon) in &mut icons {
        vis.set_if_neq(if icon.0 < reserve { Visibility::Inherited } else { Visibility::Hidden });
    }
}
