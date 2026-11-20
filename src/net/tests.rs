//! Determinism guard: two headless worlds, same seed, same inputs, one of them disturbed the way rendering
//! disturbs a real machine (extra components on sim entities, junk entities, so archetype and entity order differ).
//! Their stage hashes must agree tick for tick; a forced desync must heal through a snapshot.

use super::lockstep::step;
use super::snap;
use crate::game::{Mode, Screen, Session, Slot, Tank, Team};
use crate::geom::Dir;
use crate::input::{Device, DeviceIntents, Intent};
use crate::rng::Rng;
use bevy::prelude::*;
use bevy::state::app::StatesPlugin;
use bevy::window::AppLifecycle;

/// What gfx does to a sim entity: a component the simulation never reads.
#[derive(Component)]
struct Look;

fn world(mode: Mode, stage: usize) -> App {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, StatesPlugin))
        .add_message::<AppLifecycle>()
        .insert_resource(crate::settings::Settings::default())
        .init_resource::<DeviceIntents>()
        .init_resource::<super::Net>()
        .add_plugins(crate::game::plugin)
        .insert_resource(Rng::new(42));
    app.finish();
    app.cleanup();
    app.update();
    let mut s = app.world_mut().resource_mut::<Session>();
    s.slots = [Some(Slot::new(Device::Net(0))), Some(Slot::new(Device::Net(1)))];
    (s.mode, s.stage) = (mode, stage);
    app.world_mut().resource_mut::<NextState<Screen>>().set(Screen::Stage);
    app
}

/// Busy made-up play for player `p` at tick `t`: wander, mostly upwards, fire a lot but only up (so they don't shoot
/// their own base), A now and then (borrows a tank when out).
fn play(p: u32, t: u32) -> Intent {
    let mut r = Rng::new((t / 25) as u64 * 7 + p as u64 + 1);
    let dir = r.chance(0.85).then(|| r.pick(&[Dir::Up, Dir::Up, Dir::Left, Dir::Right, Dir::Down]));
    let up = dir == Some(Dir::Up);
    Intent { dir, fire: up && t % 30 < 18, fire_pressed: up && t.is_multiple_of(30), confirm: t % 200 == 100, ..default() }
}

/// Junk the other machine wouldn't have: re-tag some sim entities, spawn and drop loose entities.
fn disturb(world: &mut World, t: u32) {
    let mut q = world.query_filtered::<Entity, With<Tank>>();
    let tanks: Vec<Entity> = q.iter(world).collect();
    if let Some(&e) = tanks.get(t as usize % tanks.len().max(1)) {
        if t.is_multiple_of(3) {
            world.entity_mut(e).insert(Look);
        } else {
            world.entity_mut(e).remove::<Look>();
        }
    }
    let junk = world.spawn(Transform::default()).id();
    if t.is_multiple_of(2) {
        world.despawn(junk);
    }
}

fn run(mode: Mode, stage: usize, ticks: u32) -> (Vec<u64>, App, App) {
    let (mut a, mut b) = (world(mode, stage), world(mode, stage));
    let (mut ta, mut tb) = (Time::default(), Time::default());
    let mut hashes = Vec::new();
    for t in 0..ticks {
        step(a.world_mut(), &mut ta, play(0, t), play(1, t));
        disturb(b.world_mut(), t);
        step(b.world_mut(), &mut tb, play(0, t), play(1, t));
        if t % 30 == 0 {
            let (ha, hb) = (snap::hash(a.world_mut()), snap::hash(b.world_mut()));
            assert_eq!(ha, hb, "{mode:?} {stage}: worlds drifted apart by tick {t}");
            hashes.push(ha);
        }
    }
    (hashes, a, b)
}

fn live(world: &mut World) -> usize {
    world.query::<&crate::game::NetId>().iter(world).count()
}

#[test]
fn classic_stage_stays_in_step() {
    let (mut hashes, _, _) = run(Mode::Classic, 3, 1800);
    hashes.dedup();
    assert!(hashes.len() > 20, "the stage should be moving for a while, not over at once ({} states)", hashes.len());
}

#[test]
fn city_run_stays_in_step() {
    let (mut hashes, a, _) = run(Mode::City, 2, 1800);
    assert!(a.world().contains_resource::<crate::game::city::CityRun>());
    hashes.dedup();
    assert!(hashes.len() > 40, "{} states", hashes.len());
}

#[test]
fn a_desync_heals_through_the_host_snapshot() {
    let (_, mut a, mut b) = run(Mode::City, 1, 600);
    assert_eq!(*a.world().resource::<State<Screen>>().get(), Screen::Stage, "still playing");
    assert!(live(a.world_mut()) > 3, "tanks and bullets about");
    let mut q = b.world_mut().query::<(&Tank, &mut Transform)>();
    if let Some((_, mut tf)) = q.iter_mut(b.world_mut()).find(|(t, _)| t.team == Team::Player(0)) {
        tf.translation.x += 0.5;
    }
    assert_ne!(snap::hash(a.world_mut()), snap::hash(b.world_mut()), "the nudge shows");
    // What the lockstep does after a flagged tick: the host takes its snapshot and both worlds load it.
    let bytes = snap::take(a.world_mut());
    assert!(snap::apply(a.world_mut(), &bytes).is_some() && snap::apply(b.world_mut(), &bytes).is_some());
    assert_eq!(snap::take(b.world_mut()), bytes, "the snapshot is the whole stage");
    let (mut ta, mut tb) = (Time::default(), Time::default());
    for t in 600..1500 {
        step(a.world_mut(), &mut ta, play(0, t), play(1, t));
        disturb(b.world_mut(), t);
        step(b.world_mut(), &mut tb, play(0, t), play(1, t));
        assert_eq!(snap::hash(a.world_mut()), snap::hash(b.world_mut()), "back in step after the resync, tick {t}");
    }
}

#[test]
fn a_snapshot_off_the_stage_loads_nothing() {
    let mut a = world(Mode::Classic, 1);
    a.world_mut().resource_mut::<NextState<Screen>>().set(Screen::Title);
    let mut t = Time::default();
    step(a.world_mut(), &mut t, Intent::default(), Intent::default());
    let bytes = snap::take(a.world_mut());
    assert!(snap::apply(a.world_mut(), &bytes).is_none());
    assert!(snap::apply(a.world_mut(), b"\x01garbage").is_none());
}
