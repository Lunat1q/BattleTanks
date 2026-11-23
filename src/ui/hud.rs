//! In-stage HUD: side panels left and right of the field (lives, score, star level, enemy reserve or City Run
//! district track, stage).

use super::*;
use crate::game::{Mode, Screen, Session, StageRun, Tank, Team, View};
use crate::gfx::{Gfx, VIEW_HEIGHT, player_color};
use crate::terrain::Terrain;

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

/// City Run district track: the part of the district seen so far, and each player's tank on it.
#[derive(Component)]
struct TrackFill;
#[derive(Component)]
struct TrackDot(usize);

/// Track height; the boss sits at its top.
const TRACK_VH: f32 = 44.0;

/// Field width as % of screen height (the camera shows `VIEW_HEIGHT` units vertically).
const FIELD_VH: f32 = crate::geom::FIELD / VIEW_HEIGHT * 100.0;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Stage), spawn).add_systems(Update, (refresh, refresh_track).run_if(in_state(Screen::Stage)));
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
        // Basis 0: both panels split the room equally whatever their text, so a long line (A: BORROW A TANK)
        // can't widen one side and shift the field gap.
        flex_grow: 1.0,
        flex_basis: px(0),
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
            if session.mode == Mode::City {
                district_track(right, &gfx);
            } else {
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
            }
            player_block(right, 1, &gfx, &session);
        });
    });
}

/// Boss icon over a thin bar: gold up to the top of the view, a dot per player tank at its height.
fn district_track(p: &mut ChildSpawnerCommands, gfx: &Gfx) {
    p.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, row_gap: vh(1), ..default() }).with_children(
        |c| {
            c.spawn((
                ImageNode::new(gfx.boss[0].clone()).with_color(Color::srgb(0.62, 0.65, 0.72)),
                Node { width: vh(5), height: vh(5), ..default() },
            ));
            c.spawn((Node { width: vh(1), height: vh(TRACK_VH), ..default() }, BackgroundColor(DIM.with_alpha(0.35)))).with_children(
                |bar| {
                    bar.spawn((
                        Node { position_type: PositionType::Absolute, bottom: px(0), width: percent(100), height: percent(0), ..default() },
                        BackgroundColor(GOLD.with_alpha(0.7)),
                        TrackFill,
                    ));
                    for i in 0..2 {
                        bar.spawn((
                            Node {
                                position_type: PositionType::Absolute,
                                // 1P left of the bar, 2P right, so a pair side by side stays readable.
                                left: vh(if i == 0 { -2.0 } else { 1.0 }),
                                width: vh(2),
                                height: vh(1.4),
                                margin: UiRect::bottom(vh(-0.7)),
                                ..default()
                            },
                            BackgroundColor(player_color(i)),
                            Visibility::Hidden,
                            TrackDot(i),
                        ));
                    }
                },
            );
        },
    );
}

/// Height `y` on a district `height` tall, in % of the track (0 = start, 100 = top).
fn track_pct(y: f32, height: f32) -> f32 {
    if height <= 0.0 { 0.0 } else { (y / height * 100.0).clamp(0.0, 100.0) }
}

fn refresh_track(
    view: Res<View>,
    terrain: Option<Res<Terrain>>,
    tanks: Query<(&Tank, &Transform)>,
    mut fill: Query<&mut Node, (With<TrackFill>, Without<TrackDot>)>,
    mut dots: Query<(&mut Node, &mut Visibility, &TrackDot)>,
) {
    let Some(terrain) = terrain else { return };
    let h = terrain.height();
    for mut n in &mut fill {
        n.height = percent(track_pct(view.top(), h));
    }
    for (mut n, mut vis, dot) in &mut dots {
        let y = tanks.iter().find(|(t, _)| t.team == Team::Player(dot.0)).map(|(_, tf)| tf.translation.y);
        vis.set_if_neq(if y.is_some() { Visibility::Inherited } else { Visibility::Hidden });
        if let Some(y) = y {
            n.bottom = percent(track_pct(y, h));
        }
    }
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
            Hud::Level(i) => session.slots[i].as_ref().map(|s| match s.out {
                true if session.can_borrow(i) => "A: BORROW A TANK".into(),
                true => "OUT".into(),
                false => format!("POWER {}", s.level),
            }),
            Hud::Stage => Some(session.stage.to_string()),
            Hud::Hi => Some(if session.mode == Mode::Versus { String::new() } else { format!("HI {}", session.hi_score) }),
        };
        set_text(&mut t, s.unwrap_or_default());
    }
    let reserve = run.roster.len().saturating_sub(run.next_enemy);
    for (mut vis, icon) in &mut icons {
        vis.set_if_neq(if icon.0 < reserve { Visibility::Inherited } else { Visibility::Hidden });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_runs_from_the_start_to_the_boss() {
        assert_eq!(track_pct(0.0, 400.0), 0.0);
        assert_eq!(track_pct(100.0, 400.0), 25.0);
        assert_eq!(track_pct(900.0, 400.0), 100.0, "clamped at the top");
        assert_eq!(track_pct(10.0, 0.0), 0.0);
    }
}
