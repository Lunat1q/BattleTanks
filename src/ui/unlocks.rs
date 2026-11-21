//! NEW ACHIEVEMENTS: after a classic game over or a City Run district (results or game over), before the world
//! board, when the run unlocked something. The rows from the list screen arrive one by one: each slides in from
//! the right with a gold flash, its badge pops, a chime plays and the points total counts up, under a twinkling
//! title. A shows everything at once, then moves on. Skipped in LAN sessions (the screen would differ between the
//! two machines, which must change screens on the same tick).

use super::badge_rows::{self, ROW_H, RowPart, Rows, row_bg};
use super::*;
use crate::achieve::{LIST, Record};
use crate::game::{Screen, Session, Sfx};
use crate::gfx::Gfx;
use crate::input::{DeviceIntents, Intent, merge};

const VISIBLE: usize = 6;
/// First row arrives after `FIRST`, the next ones `STEP` apart; each takes `SLIDE` to land.
const FIRST: f32 = 0.7;
const STEP: f32 = 0.55;
const SLIDE: f32 = 0.35;

#[derive(Resource, Default)]
struct Reveal {
    t: f32,
    /// `LIST` indices of the run's unlocks.
    keys: Vec<usize>,
    /// Rows already arrived (each got its chime).
    shown: usize,
    scroll: usize,
}

#[derive(Component)]
struct Title;
#[derive(Component)]
struct Flourish;
#[derive(Component)]
struct Total;
#[derive(Component)]
struct Prompt;
/// A twinkling sparkle around the title: phase and speed.
#[derive(Component)]
struct Twinkle(f32, f32);

pub fn plugin(app: &mut App) {
    app.init_resource::<Reveal>()
        .add_systems(OnEnter(Screen::Unlocks), spawn)
        .add_systems(Update, (input, animate).chain().run_if(in_state(Screen::Unlocks)));
}

/// Where the game goes after a run ends: this screen if the run unlocked something, else the world board / onward.
/// An `easy` run unlocks nothing and skips the board.
pub fn after_play(rec: &Record, platform: &crate::Platform, session: &Session, linked: bool, easy: bool) -> Screen {
    if platform.achieve.is_some() && !linked && !rec.news.is_empty() {
        Screen::Unlocks
    } else {
        super::board::after_run(session, platform, linked || easy)
    }
}

fn arrives(k: usize) -> f32 {
    FIRST + k as f32 * STEP
}

/// Ease-out cubic, 0..1.
fn ease(x: f32) -> f32 {
    1.0 - (1.0 - x.clamp(0.0, 1.0)).powi(3)
}

fn spawn(mut commands: Commands, rec: Res<Record>, gfx: Res<Gfx>, mut reveal: ResMut<Reveal>) {
    let keys: Vec<usize> = rec.news.iter().filter_map(|k| LIST.iter().position(|a| a.key == *k)).collect();
    // The list is as tall as its rows (up to `VISIBLE`), so a short one sits centred under the title.
    let rows = keys.len().clamp(1, VISIBLE);
    let more = keys.len() > VISIBLE;
    *reveal = Reveal { keys, ..default() };
    let root = Node { row_gap: vh(1.2), ..screen_root() };
    commands.spawn((root, DespawnOnExit(Screen::Unlocks))).with_children(|root| {
        root.spawn(Node { flex_direction: FlexDirection::Column, align_items: AlignItems::Center, ..default() }).with_children(|head| {
            // Sparkles scattered around the title, behind it.
            for k in 0..14 {
                let (x, y) = (((k * 37 + 11) % 100) as f32, ((k * 53 + 7) % 100) as f32);
                head.spawn((
                    ImageNode::new(gfx.sparkle.clone()).with_color(GOLD),
                    Node {
                        position_type: PositionType::Absolute,
                        left: percent(x * 1.2 - 10.0),
                        top: percent(y * 1.3 - 25.0),
                        width: vh(3.0 + (k % 3) as f32 * 1.6),
                        height: vh(3.0 + (k % 3) as f32 * 1.6),
                        ..default()
                    },
                    UiTransform::IDENTITY,
                    Twinkle(k as f32 * 1.7, 2.0 + (k % 4) as f32 * 0.7),
                ));
            }
            head.spawn((text("NEW ACHIEVEMENTS", 8.0, GOLD), UiTransform::IDENTITY, Title));
            head.spawn((
                Node { width: vh(0), height: vh(0.6), margin: UiRect::top(vh(0.6)), ..default() },
                BackgroundColor(GOLD),
                Flourish,
            ));
        });
        root.spawn((text("", 3.4, GOLD), Total));
        root.spawn(Node {
            flex_direction: FlexDirection::Column,
            height: vh(ROW_H * rows as f32),
            margin: UiRect::top(vh(1.0)),
            ..default()
        })
        .with_children(|list| {
            for i in 0..rows {
                badge_rows::spawn(list, i, vh(140));
            }
        });
        let prompt = if more { "press A to continue      up/down  more" } else { "press A to continue" };
        root.spawn((text(prompt, 3.0, DIM), Visibility::Hidden, Prompt));
    });
}

fn input(
    time: Res<Time>,
    mut reveal: ResMut<Reveal>,
    session: Res<Session>,
    devices: Res<DeviceIntents>,
    platform: Res<crate::Platform>,
    net: Res<crate::net::Net>,
    settings: Res<crate::settings::Settings>,
    mut next: ResMut<NextState<Screen>>,
    mut sfx: MessageWriter<Sfx>,
) {
    reveal.t += time.delta_secs();
    let n = reveal.keys.len();
    let all_in = arrives(n.saturating_sub(1)) + SLIDE;
    let i: Intent = (0..2).map(|s| session.intent(s, &devices)).fold(Intent::default(), merge);
    if i.confirm || i.start || i.back {
        if reveal.t < all_in {
            reveal.t = all_in; // show the rest at once
        } else if reveal.t > all_in + 0.4 {
            sfx.write(Sfx::MenuPick);
            next.set(super::board::after_run(&session, &platform, net.linked() || settings.easy_mode));
        }
    }
    // A chime for each row as it lands; the list follows the newest one, then up/down browses it.
    let arrived = (0..n).filter(|&k| reveal.t >= arrives(k)).count();
    if arrived > reveal.shown {
        reveal.shown = arrived;
        reveal.scroll = arrived.saturating_sub(VISIBLE);
        sfx.write(if arrived == n { Sfx::ExtraLife } else { Sfx::PowerPick });
    } else if arrived == n {
        let last = n.saturating_sub(VISIBLE);
        match i.nav {
            Some(crate::geom::Dir::Up) => reveal.scroll = reveal.scroll.saturating_sub(1),
            Some(crate::geom::Dir::Down) => reveal.scroll = (reveal.scroll + 1).min(last),
            _ => {}
        }
    }
}

fn animate(
    reveal: Res<Reveal>,
    rec: Res<Record>,
    mut rows: Rows,
    mut title: Query<&mut UiTransform, (With<Title>, Without<Twinkle>, Without<RowPart>)>,
    mut flourish: Query<&mut Node, (With<Flourish>, Without<RowPart>)>,
    mut twinkles: Query<(&Twinkle, &mut ImageNode, &mut UiTransform), (Without<Title>, Without<RowPart>)>,
    mut total: Query<&mut Text, (With<Total>, Without<RowPart>)>,
    mut prompt: Query<&mut Visibility, (With<Prompt>, Without<RowPart>)>,
) {
    let t = reveal.t;
    let n = reveal.keys.len();
    for slot in 0..VISIBLE {
        let k = reveal.scroll + slot;
        let entry = reveal.keys.get(k).copied().filter(|_| t >= arrives(k));
        rows.show(slot, entry, &rec);
        let age = t - arrives(k);
        // Slide in from the right, flash gold, then settle; the badge pops past full size and back.
        for (b, mut bg, _, _, mut tf) in &mut rows.boxes {
            if b.0 == slot {
                tf.translation = Val2::new(Val::Vw(60.0 * (1.0 - ease(age / SLIDE))), Val::Px(0.0));
                let flash = (1.0 - (age - SLIDE * 0.6) / 0.6).clamp(0.0, 1.0);
                bg.0 = row_bg(slot).mix(&Color::srgb(0.55, 0.42, 0.14), flash * 0.8);
            }
        }
        for (r, _, mut tf) in &mut rows.icons {
            if r.0 == slot {
                let s = if age < SLIDE {
                    0.6
                } else {
                    1.0 + 0.45 * (1.0 - (age - SLIDE) / 0.3).clamp(0.0, 1.0) * ((age - SLIDE) * 10.0).min(1.0)
                };
                tf.scale = Vec2::splat(s);
            }
        }
    }
    if let Ok(mut tf) = title.single_mut() {
        let pop = ease(t / 0.45);
        tf.scale = Vec2::splat((0.6 + 0.4 * pop) * (1.0 + 0.025 * (t * 3.0).sin()));
    }
    if let Ok(mut node) = flourish.single_mut() {
        node.width = vh(70.0 * ease(t / 0.8));
    }
    for (tw, mut img, mut tf) in &mut twinkles {
        let w = ((t * tw.1 + tw.0).sin() * 0.5 + 0.5).powi(3);
        img.color = GOLD.with_alpha(w * ease(t / 0.6));
        tf.scale = Vec2::splat(0.4 + 0.8 * w);
        tf.rotation = Rot2::radians(t * 0.8 + tw.0);
    }
    let landed: u32 = reveal.keys.iter().enumerate().filter(|&(k, _)| t >= arrives(k) + SLIDE).map(|(_, &i)| LIST[i].pts).sum();
    if let Ok(mut s) = total.single_mut() {
        let got = reveal.keys.iter().enumerate().filter(|&(k, _)| t >= arrives(k)).count();
        set_text(&mut s, format!("+{landed} POINTS   |   {got} NEW   |   {} / {} UNLOCKED", rec.done.len(), LIST.len()));
    }
    if let Ok(mut v) = prompt.single_mut() {
        let ready = t > arrives(n.saturating_sub(1)) + SLIDE + 0.4;
        v.set_if_neq(if ready && (t * 2.0).fract() < 0.7 { Visibility::Inherited } else { Visibility::Hidden });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_arrive_in_order_and_ease_lands() {
        assert!(arrives(0) < arrives(1) && arrives(1) - arrives(0) == STEP);
        assert_eq!((ease(0.0), ease(1.0), ease(2.0)), (0.0, 1.0, 1.0));
        assert!(ease(0.5) > 0.5, "fast start, soft landing");
    }
}
