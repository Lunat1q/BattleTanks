//! City Run district map: ten district dots joined by dotted roads. Cleared districts glow gold,
//! the next one is open, the rest are locked. Left/right (or up/down) picks, A starts, B goes back.

use super::*;
use crate::game::progress::Progress;
use crate::game::{Mode, Screen, Session, Sfx};
use crate::geom::Dir;
use crate::gfx::Gfx;
use crate::input::DeviceIntents;
use crate::rules::CITY_DISTRICTS;

/// Dot centres in % of the map panel (x, y from the top), a road winding from bottom-left to top-right.
const SPOTS: [(f32, f32); CITY_DISTRICTS] = [
    (7.0, 82.0),
    (21.0, 64.0),
    (13.0, 36.0),
    (29.0, 16.0),
    (45.0, 32.0),
    (51.0, 64.0),
    (66.0, 82.0),
    (79.0, 58.0),
    (70.0, 28.0),
    (90.0, 14.0),
];
const ROAD_DOTS: usize = 7;

#[derive(Resource, Default)]
struct Sel(usize);

#[derive(Component)]
struct DistrictDot(usize);
#[derive(Component)]
struct DotLabel(usize);
/// Road dot between district `n` and `n + 1`.
#[derive(Component)]
struct RoadDot(usize);
#[derive(Component)]
struct Info;

pub fn plugin(app: &mut App) {
    app.init_resource::<Sel>()
        .add_systems(OnEnter(Screen::CityMap), spawn)
        .add_systems(Update, (input, refresh).chain().run_if(in_state(Screen::CityMap)));
}

fn spawn(
    mut commands: Commands,
    gfx: Res<Gfx>,
    progress: Res<Progress>,
    settings: Res<crate::settings::Settings>,
    mut sel: ResMut<Sel>,
    mut heads: Headings,
) {
    sel.0 = progress.open(Mode::City, settings.easy_mode).min(CITY_DISTRICTS);
    let at = |(x, y): (f32, f32), size: f32| Node {
        position_type: PositionType::Absolute,
        left: percent(x),
        top: percent(y),
        width: vh(size),
        height: vh(size),
        margin: UiRect { left: vh(-size / 2.0), top: vh(-size / 2.0), ..default() },
        border_radius: BorderRadius::MAX,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        ..default()
    };
    commands.spawn((screen_root(), DespawnOnExit(Screen::CityMap))).with_children(|root| {
        root.spawn(heads.node("CITY RUN", Block::Steel, 7.5));
        root.spawn((
            Node { width: vh(130), height: vh(56), border_radius: BorderRadius::all(vh(3)), ..default() },
            ImageNode::new(gfx.city_backdrop.clone()),
        ))
        .with_children(|map| {
            for n in 0..CITY_DISTRICTS - 1 {
                let (a, b) = (SPOTS[n], SPOTS[n + 1]);
                for k in 1..=ROAD_DOTS {
                    let t = k as f32 / (ROAD_DOTS + 1) as f32;
                    let p = (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t);
                    map.spawn((
                        Node { border: UiRect::all(vh(0.25)), ..at(p, 1.4) },
                        BorderColor::all(Color::srgb(0.03, 0.03, 0.04)),
                        BackgroundColor(DIM),
                        RoadDot(n),
                    ));
                }
            }
            for (n, &p) in SPOTS.iter().enumerate() {
                map.spawn((
                    Node { border: UiRect::all(vh(0.45)), ..at(p, 6.5) },
                    BorderColor::all(Color::srgb(0.03, 0.03, 0.04)),
                    BackgroundColor(DIM),
                    Outline::new(vh(0.5), vh(0.6), Color::NONE),
                    DistrictDot(n + 1),
                ))
                .with_child((text((n + 1).to_string(), 3.2, INK), DotLabel(n + 1)));
            }
        });
        root.spawn((text("", 4.6, INK), Info));
        root.spawn(text("left/right choose   A start   B back   -   power-ups reset every district", 2.4, DIM));
    });
}

fn input(
    devices: Res<DeviceIntents>,
    progress: Res<Progress>,
    settings: Res<crate::settings::Settings>,
    mut session: ResMut<Session>,
    mut sel: ResMut<Sel>,
    mut next: ResMut<NextState<Screen>>,
    mut sfx: MessageWriter<Sfx>,
) {
    let nav = devices.0.iter().map(|(_, i)| *i).fold(Default::default(), crate::input::merge);
    let last = progress.open(Mode::City, settings.easy_mode).min(CITY_DISTRICTS);
    let moved = match nav.nav {
        Some(Dir::Left | Dir::Down) if sel.0 > 1 => Some(sel.0 - 1),
        Some(Dir::Right | Dir::Up) if sel.0 < last => Some(sel.0 + 1),
        _ => None,
    };
    if let Some(n) = moved {
        sel.0 = n;
        sfx.write(Sfx::MenuMove);
    }
    if nav.back {
        next.set(Screen::Play);
    } else if (nav.confirm || nav.start) && session.players() > 0 {
        session.mode = Mode::City;
        session.stage = sel.0;
        sfx.write(Sfx::MenuPick);
        next.set(Screen::Stage);
    }
}

fn refresh(
    time: Res<Time>,
    progress: Res<Progress>,
    settings: Res<crate::settings::Settings>,
    sel: Res<Sel>,
    session: Res<Session>,
    best: Res<crate::scores::Best>,
    mut dots: Query<(&DistrictDot, &mut BackgroundColor, &mut Outline), Without<RoadDot>>,
    mut roads: Query<(&RoadDot, &mut BackgroundColor), Without<DistrictDot>>,
    mut labels: Query<(&DotLabel, &mut TextColor)>,
    mut info: Query<&mut Text, With<Info>>,
) {
    let open = progress.open(Mode::City, settings.easy_mode);
    let locked = Color::srgb(0.20, 0.22, 0.27);
    let pulse = 0.75 + 0.25 * (time.elapsed_secs() * 5.0).sin();
    for (dot, mut bg, mut outline) in &mut dots {
        bg.0 = match dot.0 {
            n if n < open => GOLD,
            n if n == open => INK,
            _ => locked,
        };
        outline.color = if dot.0 == sel.0 { Color::WHITE.with_alpha(pulse) } else { Color::NONE };
    }
    for (road, mut bg) in &mut roads {
        bg.0 = if road.0 + 2 <= open { GOLD } else { DIM };
    }
    for (label, mut color) in &mut labels {
        color.0 = if label.0 <= open { Color::srgb(0.08, 0.08, 0.1) } else { DIM };
    }
    if let Ok(mut t) = info.single_mut() {
        let status = match best.city_time(session.players(), sel.0) {
            Some(t) => format!("   BEST {}", crate::scores::clock_text(t)),
            None if sel.0 < open => "   CLEARED".into(),
            None => String::new(),
        };
        set_text(&mut t, format!("DISTRICT {}  {}{status}", sel.0, crate::city_map::name(sel.0).to_uppercase()));
    }
}
