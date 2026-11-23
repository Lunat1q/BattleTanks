//! LAN co-op (ADR 0014): two machines on one network play one game in lockstep. The host broadcasts a beacon, the
//! joiner connects; then each tick both send their player's packed `Intent`, and a tick runs only once both are in,
//! with the same fixed `dt`, so both simulations stay the same. Every few ticks they compare a hash of the stage; if
//! it differs, both load the host's snapshot after the same tick (`snap`).
//!
//! The `Driver` schedule takes `Update`'s place in the main loop: offline it runs `Update` once per frame (nothing
//! changes), linked it runs `StateTransition` + `Update` once per lockstep tick, 0-4 times a frame.

pub mod link;
mod lockstep;
pub mod snap;
#[cfg(test)]
mod tests;
mod versus;
pub mod wire;

use crate::game::progress::{OwnProgress, Progress};
use crate::game::{Paused, Screen, Session};
use crate::input::{Device, DeviceIntents, Intent};
use crate::settings::Settings;
use bevy::app::MainScheduleOrder;
use bevy::ecs::schedule::ScheduleLabel;
use bevy::prelude::*;
use bevy::window::AppLifecycle;
use link::{Beacon, Conn, Finder, Hello};
use lockstep::Lockstep;
use std::time::Instant;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
struct Driver;

/// Before the link: hosting (beacon out) or shaking hands.
#[derive(Default)]
enum Pending {
    #[default]
    None,
    Hosting(Beacon),
    /// Host: a joiner connected, its hello is due.
    Greeting(Conn, Instant),
    /// Joiner: our hello is out, the host's is due.
    Joining(Conn, Instant),
}

#[derive(Resource, Default)]
pub struct Net {
    pending: Pending,
    /// Hosts heard while the LAN screen is open.
    pub finder: Option<Finder>,
    /// This machine's name (from `Platform::lan`).
    pub name: String,
    /// What went wrong last, for the LAN screen.
    pub note: String,
    /// Player slot of this machine while linked: 0 host, 1 joiner.
    me: Option<u8>,
    pub names: [String; 2],
    /// One-shot presses gathered since our last packed tick.
    pending_intent: Intent,
    /// Set by a menu on a lockstep tick (so on both machines): the session ends after it.
    pub leave: bool,
    /// Seconds since the last tick ran while linked (the partner is slow or gone).
    pub stalled: f32,
    /// While linked: (round trip p95 ms, our input delay in ticks), once measured.
    pub ping: Option<(u32, u32)>,
    /// A finished handshake for the driver, which needs the whole world to start the session.
    ready: Option<(Conn, Hello, u8, Hello)>,
    /// This machine's first local device: it takes over our slot when the session ends.
    local: Option<Device>,
}

impl Net {
    pub fn linked(&self) -> bool {
        self.me.is_some()
    }
    pub fn me(&self) -> Option<u8> {
        self.me
    }
    pub fn hosting(&self) -> bool {
        matches!(self.pending, Pending::Hosting(_) | Pending::Greeting(..))
    }
    pub fn joining(&self) -> bool {
        matches!(self.pending, Pending::Joining(..))
    }

    /// Starts listening for hosts (the LAN screen's list).
    pub fn listen(&mut self) {
        if self.finder.is_none() {
            match Finder::new() {
                Ok(f) => self.finder = Some(f),
                Err(e) => self.note = format!("can't look for games here: {e}"),
            }
        }
    }

    /// Starts hosting; stops listening, so a second copy on this machine can take the beacon port.
    pub fn host(&mut self) {
        self.finder = None;
        match Beacon::new(VERSION, &self.name) {
            Ok(b) => (self.pending, self.note) = (Pending::Hosting(b), String::new()),
            Err(e) => self.note = format!("can't host: {e}"),
        }
    }

    pub fn join(&mut self, i: usize) {
        let Some(f) = self.finder.as_ref().and_then(|f| f.found.get(i)) else { return };
        if f.version != VERSION {
            self.note = format!("{} runs v{}, this is v{VERSION}", f.name, f.version);
            return;
        }
        let hello = self.hello(0);
        match Conn::connect(f.addr) {
            Ok(mut c) => {
                let udp = c.udp_port();
                if c.hello(&Hello { udp, ..hello }) {
                    (self.pending, self.note) = (Pending::Joining(c, Instant::now()), String::new());
                } else {
                    self.note = "the host hung up".into();
                }
            }
            Err(e) => self.note = format!("can't reach {}: {e}", f.name),
        }
    }

    pub fn cancel(&mut self) {
        self.pending = Pending::None;
    }

    fn hello(&self, udp: u16) -> Hello {
        Hello { version: VERSION.into(), name: self.name.clone(), udp, ..default() }
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Net>()
        .init_schedule(Driver)
        .add_systems(Driver, drive)
        .add_systems(PreUpdate, (shake_hands, collect.after(crate::input::gather)))
        .add_systems(Startup, lockstep::spawn_overlay)
        .add_systems(PostUpdate, lockstep::overlay);
    let mut order = app.world_mut().resource_mut::<MainScheduleOrder>();
    let i = order.labels.iter().position(|l| (**l).eq(&Update)).expect("Update in the main schedule");
    order.labels[i] = Driver.intern();
}

/// Offline: `Update` once. Linked: the lockstep decides how many ticks run this frame.
fn drive(world: &mut World) {
    if let Some(ready) = world.resource_mut::<Net>().ready.take() {
        start(world, ready);
    }
    if !world.contains_resource::<Lockstep>() {
        world.run_schedule(Update);
        return;
    }
    let end = world.resource_scope(|world, mut ls: Mut<Lockstep>| ls.frame(world));
    let leave = std::mem::take(&mut world.resource_mut::<Net>().leave);
    if let Some(why) = end.or(leave.then(String::new)) {
        finish(world, why);
    }
}

/// While linked: this machine's devices, merged, become our player's input (`Lockstep` packs it per tick).
fn collect(mut net: ResMut<Net>, devices: Res<DeviceIntents>, screen: Res<State<Screen>>, mut lifecycle: MessageReader<AppLifecycle>) {
    let local = devices.0.iter().map(|(d, _)| *d);
    net.local = local.clone().find(|d| matches!(d, Device::Pad(_))).or(local.clone().next());
    if !net.linked() {
        lifecycle.clear();
        return;
    }
    let mut now =
        devices.0.iter().filter(|(d, _)| !matches!(d, Device::Net(_))).fold(Intent::default(), |a, (_, i)| crate::input::merge(a, *i));
    now.away = lifecycle.read().any(|&e| crate::game::stage::backgrounded(e) == Some(true));
    if let Some(bot) = crate::dev::lan_bot(net.me, *screen.get()) {
        now = bot;
    }
    net.pending_intent = wire::gather(net.pending_intent, now);
}

/// Hosting / joining progress, every frame until linked.
fn shake_hands(mut net: ResMut<Net>, settings: Res<Settings>, progress: Res<Progress>) {
    if let Some(f) = net.finder.as_mut() {
        f.poll();
    }
    let wait = std::time::Duration::from_secs(5);
    match std::mem::take(&mut net.pending) {
        Pending::Hosting(mut b) => match b.poll().map(Conn::new) {
            Some(Ok(c)) => net.pending = Pending::Greeting(c, Instant::now()),
            Some(Err(e)) => (net.pending, net.note) = (Pending::Hosting(b), format!("join failed: {e}")),
            None => net.pending = Pending::Hosting(b),
        },
        Pending::Greeting(mut c, since) => match c.recv() {
            Some((wire::HELLO, body)) => match Hello::decode(&body) {
                Some(theirs) => {
                    let ours = Hello {
                        udp: c.udp_port(),
                        session: rand32(),
                        seed: crate::rng::Rng::from_time().0,
                        friendly_fire: settings.friendly_fire,
                        easy_mode: settings.easy_mode,
                        // What this host may start, so the joiner's copy opens the same tiles.
                        unlocked: progress.open(crate::game::Mode::City, settings.easy_mode) as u8,
                        classic: progress.open(crate::game::Mode::Classic, settings.easy_mode) as u8,
                        ..net.hello(0)
                    };
                    // Sent even on a version mismatch, so the joiner can say which version we run.
                    if c.hello(&ours) && theirs.version == VERSION {
                        c.greeted(&theirs);
                        net.ready = Some((c, theirs, 0, ours));
                    } else {
                        net.host(); // keep waiting for a joiner that fits
                    }
                }
                None => net.host(),
            },
            Some(_) => net.host(),
            None if since.elapsed() > wait => net.host(),
            None => net.pending = Pending::Greeting(c, since),
        },
        Pending::Joining(mut c, since) => match c.recv() {
            Some((wire::HELLO, body)) => match Hello::decode(&body) {
                Some(theirs) if theirs.version == VERSION => {
                    c.greeted(&theirs);
                    let ours = Hello { udp: c.udp_port(), ..net.hello(0) };
                    net.ready = Some((c, theirs, 1, ours));
                }
                Some(theirs) => net.note = format!("{} runs v{}, this is v{VERSION}", theirs.name, theirs.version),
                None => net.note = "that host speaks another protocol".into(),
            },
            Some(_) => net.note = "the host hung up".into(),
            None if since.elapsed() > wait => net.note = "the host didn't answer".into(),
            None => net.pending = Pending::Joining(c, since),
        },
        Pending::None => {}
    }
}

fn rand32() -> u32 {
    crate::rng::Rng::from_time().next_u32()
}

/// Both machines start from the host's state, on the title, with nobody joined.
fn start(world: &mut World, (conn, theirs, me, ours): (Conn, Hello, u8, Hello)) {
    let host = if me == 0 { &ours } else { &theirs };
    let (seed, session, ff, keep) = (host.seed, host.session, host.friendly_fire, host.easy_mode);
    let hosts = Progress { unlocked: host.unlocked as usize, classic: host.classic as usize, ..Progress::default() };
    let names = if me == 0 { [ours.name.clone(), theirs.name.clone()] } else { [theirs.name.clone(), ours.name.clone()] };
    info!("lan: linked as player {} ({} + {}), session {session:08x}", me + 1, names[0], names[1]);
    let mut settings = world.resource_mut::<Settings>();
    let restore = (settings.friendly_fire, settings.easy_mode);
    (settings.friendly_fire, settings.easy_mode) = (ff, keep);
    let own = std::mem::replace(&mut *world.resource_mut::<Progress>(), hosts);
    world.resource_mut::<OwnProgress>().0 = (me == 1).then_some(own);
    world.insert_resource(crate::rng::Rng::new(seed));
    let mut s = world.resource_mut::<Session>();
    (s.slots, s.mode, s.stage, s.bonus) = ([None, None], crate::game::Mode::Classic, 1, 0);
    s.log.clear();
    world.insert_resource(crate::ui::title::TitleMenu::default());
    crate::ui::reset_menus(world);
    world.resource_mut::<Paused>().0 = false;
    world.resource_mut::<NextState<Screen>>().set(Screen::Title);
    world.insert_resource(Lockstep::new(conn, me, session, restore));
    let mut net = world.resource_mut::<Net>();
    (net.me, net.names, net.pending, net.note, net.stalled, net.ping) = (Some(me), names, Pending::None, String::new(), 0.0, None);
    net.pending_intent = Intent::default();
}

/// Back to playing alone. Mid-run this machine's player keeps going on a local device and the partner's slot
/// shows as a lost controller (pause, any pad takes over); on a menu both slots empty out and the title comes up.
fn finish(world: &mut World, why: String) {
    if let Some(mut ls) = world.remove_resource::<Lockstep>() {
        ls.bye();
        let (ff, keep) = ls.restore;
        if ls.me == 1 {
            let mut settings = world.resource_mut::<Settings>();
            (settings.friendly_fire, settings.easy_mode) = (ff, keep);
        }
        if let Some(own) = world.resource_mut::<OwnProgress>().0.take() {
            *world.resource_mut::<Progress>() = own;
        }
    }
    let mut net = world.resource_mut::<Net>();
    let me = net.me.take().unwrap_or(0);
    let local = net.local;
    info!("lan: session over{}", if why.is_empty() { String::new() } else { format!(": {why}") });
    net.note = why;
    let screen = *world.resource::<State<Screen>>().get();
    let mut session = world.resource_mut::<Session>();
    if matches!(screen, Screen::Title | Screen::Lan) {
        session.slots = [None, None];
        world.resource_mut::<NextState<Screen>>().set(Screen::Title);
    } else if let Some(slot) = session.slots.iter_mut().flatten().find(|s| s.device == Device::Net(me)) {
        slot.device = local.unwrap_or(Device::KeysLeft);
    }
    // The last lockstep tick's intents would read as held buttons until `gather` runs again.
    world.resource_mut::<DeviceIntents>().0.clear();
}
