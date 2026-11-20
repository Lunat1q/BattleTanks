//! The running session: ticks, input exchange, hash checks, resync, and the "waiting for partner" overlay.

use super::link::Conn;
use super::wire::{self, Packet, RESYNC};
use super::{Net, snap};
use crate::game::Screen;
use crate::input::{Device, DeviceIntents};
use bevy::prelude::*;
use bevy::state::state::StateTransition;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// Simulation step while linked.
const TICK: f64 = 1.0 / 60.0;
/// Our input for tick `t` is made at tick `t - DELAY`: 50 ms for the packet to cross. Calibration knob (Wi-Fi with
/// spikes may want 4-5; every extra tick is felt by both players).
const DELAY: u32 = 3;
/// Ticks between state hashes (half a second).
const HASH_EVERY: u32 = 30;
/// Silence after which the partner counts as gone (a phone call in the background is fine up to here).
const TIMEOUT: Duration = Duration::from_secs(45);
/// Most ticks a frame may run to catch up.
const MAX_CATCH_UP: f64 = 4.0;

#[derive(Resource)]
pub struct Lockstep {
    pub me: u8,
    conn: Conn,
    session: u32,
    /// Next tick to simulate.
    tick: u32,
    /// Our inputs by tick, kept until the peer acknowledged them.
    own: BTreeMap<u32, u16>,
    /// The peer's inputs by tick, dropped once simulated.
    theirs: BTreeMap<u32, u16>,
    /// Next tick our input goes to (= `tick + DELAY`).
    own_next: u32,
    /// The peer has all our inputs below this.
    peer_acked: u32,
    /// We have all the peer's inputs below this.
    peer_next: u32,
    /// Ticks owed to real time.
    due: f64,
    time: Time,
    /// Our recent hashes by tick.
    hashes: BTreeMap<u32, u64>,
    peer_hash: Option<(u32, u64)>,
    /// Host: compare only hashes after this tick (earlier ones predate the last resync).
    trust_from: u32,
    /// Host: put `RESYNC` on our next input.
    resync_next: bool,
    /// Joiner: the host's snapshot is due before the next tick.
    waiting: bool,
    snapshot: Option<Vec<u8>>,
    heard: Instant,
    /// The joiner's own friendly fire, keep power and City Run unlocks, put back afterwards.
    pub restore: (bool, bool, usize),
    resyncs: u32,
}

impl Lockstep {
    pub fn new(conn: Conn, me: u8, session: u32, restore: (bool, bool, usize)) -> Self {
        // The first DELAY ticks have no input from anyone; both sides know that without asking.
        let blank: BTreeMap<u32, u16> = (0..DELAY).map(|t| (t, 0)).collect();
        Lockstep {
            me,
            conn,
            session,
            tick: 0,
            own: blank.clone(),
            theirs: blank,
            own_next: DELAY,
            peer_acked: DELAY,
            peer_next: DELAY,
            due: 0.0,
            time: Time::default(),
            hashes: BTreeMap::new(),
            peer_hash: None,
            trust_from: 0,
            resync_next: false,
            waiting: false,
            snapshot: None,
            heard: Instant::now(),
            restore,
            resyncs: 0,
        }
    }

    pub fn bye(&mut self) {
        self.conn.send(wire::BYE, &[]);
    }

    /// One rendered frame: read the wire, run the ticks that are ready and due, answer. `Some(reason)` ends it.
    pub fn frame(&mut self, world: &mut World) -> Option<String> {
        if let Err(why) = self.receive() {
            return Some(why);
        }
        let real = world.resource::<Time<Real>>().delta_secs_f64();
        self.due = (self.due + real / TICK).min(MAX_CATCH_UP);
        let mut ran = 0;
        loop {
            if self.waiting {
                let Some(bytes) = self.snapshot.take() else { break };
                if snap::apply(world, &bytes).is_none() {
                    return Some("lost sync".into());
                }
                self.waiting = false;
                self.resyncs += 1;
                info!("lan: resync {} loaded after tick {}", self.resyncs, self.tick - 1);
            }
            if self.due < 1.0 || world.resource::<Net>().leave {
                break;
            }
            let (Some(&a), Some(&b)) = (self.own.get(&self.tick), self.theirs.get(&self.tick)) else { break };
            let [host, joiner] = if self.me == 0 { [a, b] } else { [b, a] };
            self.pack_next(world);
            self.run(world, host, joiner);
            if host & RESYNC != 0 {
                if self.me == 0 {
                    // Load our own copy too: both worlds then rebuild every entity in the same order.
                    let bytes = snap::take(world);
                    if snap::apply(world, &bytes).is_none() || !self.conn.send(wire::SNAPSHOT, &bytes) {
                        return Some("lost sync".into());
                    }
                    self.resyncs += 1;
                    info!("lan: resync {} sent after tick {} ({} bytes)", self.resyncs, self.tick, bytes.len());
                } else {
                    self.waiting = true;
                }
            }
            if self.tick.is_multiple_of(HASH_EVERY) {
                let h = snap::hash(world);
                self.hashes.insert(self.tick, h);
                if crate::dev::lan_log() {
                    let screen = *world.resource::<State<Screen>>().get();
                    let s = world.resource::<crate::game::Session>();
                    let slots: Vec<_> = s.slots.iter().flatten().map(|p| (p.lives, p.score, p.out)).collect();
                    info!("lan: tick {} hash {h:016x} {screen:?} {:?} stage {} log {} {slots:?}", self.tick, s.mode, s.stage, s.log.len());
                }
            }
            self.tick += 1;
            self.due -= 1.0;
            ran += 1;
        }
        let tick = self.tick;
        self.theirs.retain(|&t, _| t >= tick);
        let keep = self.peer_acked.min(tick);
        self.own.retain(|&t, _| t >= keep);
        self.hashes.retain(|&t, _| t + 20 * HASH_EVERY >= tick);
        if self.me == 0
            && let Some(why) = self.check(world)
        {
            return Some(why);
        }
        self.send();
        let mut net = world.resource_mut::<Net>();
        net.stalled = if ran > 0 { 0.0 } else { net.stalled + real as f32 };
        (self.heard.elapsed() > TIMEOUT).then(|| "lost the connection".into())
    }

    /// Our player's input for tick `own_next`, from what `collect` gathered since the last one.
    fn pack_next(&mut self, world: &mut World) {
        let mut net = world.resource_mut::<Net>();
        let intent = net.pending_intent;
        net.pending_intent = wire::held(intent);
        let mut bits = wire::pack(&intent);
        if self.me == 0 && std::mem::take(&mut self.resync_next) {
            bits |= RESYNC;
            self.trust_from = self.own_next + 1;
        }
        self.own.insert(self.own_next, bits);
        self.own_next += 1;
    }

    fn run(&mut self, world: &mut World, host: u16, joiner: u16) {
        crate::dev::lan_nudge(world, self.me, self.tick);
        step(world, &mut self.time, wire::unpack(host), wire::unpack(joiner));
    }

    fn receive(&mut self) -> Result<(), String> {
        let mut buf = [0u8; 1500];
        while let Some(n) = self.conn.recv_udp(&mut buf) {
            let Some(p) = Packet::decode(&buf[..n]).filter(|p| p.session == self.session) else { continue };
            self.heard = Instant::now();
            for (k, &bits) in p.inputs.iter().enumerate() {
                let t = p.first + k as u32;
                if t >= self.tick {
                    self.theirs.insert(t, bits);
                }
            }
            while self.peer_next < self.tick || self.theirs.contains_key(&self.peer_next) {
                self.peer_next += 1;
            }
            self.peer_acked = self.peer_acked.max(p.ack);
            if p.hash.is_some_and(|(t, _)| self.peer_hash.is_none_or(|(old, _)| t > old)) {
                self.peer_hash = p.hash;
            }
        }
        while let Some((kind, body)) = self.conn.recv() {
            match kind {
                wire::SNAPSHOT => self.snapshot = Some(body),
                wire::BYE => return Err("the partner left".into()),
                _ => {}
            }
        }
        Ok(())
    }

    /// Host: a hash that differs from ours at the same tick asks for a resync (on a stage) or ends the session.
    fn check(&mut self, world: &World) -> Option<String> {
        let (t, theirs) = self.peer_hash?;
        let ours = *self.hashes.get(&t)?;
        self.peer_hash = None;
        if t < self.trust_from || ours == theirs {
            return None;
        }
        warn!("lan: hashes differ at tick {t} ({ours:016x} here, {theirs:016x} there)");
        if *world.resource::<State<Screen>>().get() != Screen::Stage {
            return Some("lost sync".into());
        }
        self.resync_next = true;
        self.trust_from = u32::MAX; // until the flagged tick is chosen
        None
    }

    fn send(&mut self) {
        let first = self.peer_acked.min(self.own_next);
        let inputs: Vec<u16> = (first..self.own_next).take(64).map(|t| self.own.get(&t).copied().unwrap_or(0)).collect();
        let hash = self.hashes.last_key_value().map(|(&t, &h)| (t, h));
        self.conn.send_udp(&Packet { session: self.session, first, inputs, ack: self.peer_next, hash }.encode());
    }
}

/// One lockstep tick: both players' intents, a fixed `dt`, any screen change, then the game's `Update`.
pub fn step(world: &mut World, time: &mut Time, host: crate::input::Intent, joiner: crate::input::Intent) {
    world.resource_mut::<DeviceIntents>().0 = vec![(Device::Net(0), host), (Device::Net(1), joiner)];
    time.advance_by(Duration::from_secs_f64(TICK));
    *world.resource_mut::<Time>() = *time;
    world.run_schedule(StateTransition);
    world.run_schedule(Update);
}

#[derive(Component)]
pub struct WaitNote;

pub fn spawn_overlay(mut commands: Commands) {
    commands.spawn((
        crate::ui::text("", 4.0, crate::ui::GOLD),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Vh(44.0),
            width: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            ..default()
        },
        GlobalZIndex(100),
        WaitNote,
    ));
}

/// "waiting for <partner>" once ticks stop for a moment.
pub fn overlay(net: Res<Net>, mut note: Query<&mut Text, With<WaitNote>>) {
    let Ok(mut t) = note.single_mut() else { return };
    let msg = match net.me() {
        Some(me) if net.stalled > 0.4 => format!("waiting for {}...", net.names[1 - me as usize]),
        _ => String::new(),
    };
    if t.0 != msg {
        t.0 = msg;
    }
}
