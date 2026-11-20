//! The live stage as bytes. Both machines hash it every few ticks; when the hashes differ, both load the host's
//! copy after the same tick (`net::lockstep`). Loading respawns every simulated entity in the snapshot's order, so
//! the two worlds also iterate alike again. Outside a stage it is just the screen and the session, enough to notice
//! that two machines are on different screens (which a snapshot can't fix: the session ends).

use super::wire::{R, W, index};
use crate::city_map::Spawn;
use crate::game::city::CityRun;
use crate::game::*;
use crate::geom::Dir;
use crate::input::Device;
use crate::rng::Rng;
use crate::rules::{EnemyKind, GunStats, PowerKind, Rank};
use crate::terrain::{Terrain, Tile};
use bevy::prelude::*;

const SCREENS: [Screen; 9] = [
    Screen::Title,
    Screen::Stage,
    Screen::Results,
    Screen::GameOver,
    Screen::Settings,
    Screen::CityMap,
    Screen::Board,
    Screen::HiScores,
    Screen::Lan,
];
const RANKS: [Rank; 4] = [Rank::Normal, Rank::Turret, Rank::Mini, Rank::Boss];
const PHASES: [Phase; 4] = [Phase::Intro, Phase::Play, Phase::Won, Phase::Lost];
const MODES: [Mode; 2] = [Mode::Classic, Mode::City];
/// No entity (a dead bullet owner, no boss yet).
const NONE: u32 = u32::MAX;

pub fn hash(world: &mut World) -> u64 {
    super::wire::fnv(&take(world))
}

/// The screen, the session and, on a live stage, everything the simulation reads.
pub fn take(world: &mut World) -> Vec<u8> {
    let mut w = W::default();
    let screen = *world.resource::<State<Screen>>().get();
    w.u8(index(&SCREENS, &screen));
    put_session(&mut w, world.resource::<Session>());
    if screen != Screen::Stage || !world.contains_resource::<StageRun>() || !world.contains_resource::<Terrain>() {
        return w.0;
    }
    let mut q = world.query::<(Entity, &NetId)>();
    let mut ents: Vec<(NetId, Entity)> = q.iter(world).map(|(e, id)| (*id, e)).collect();
    ents.sort();
    let slot = |e: Entity| ents.iter().position(|(_, x)| *x == e).map_or(NONE, |i| i as u32);

    w.u64(world.resource::<Rng>().0).bool(world.resource::<Paused>().0).f32(world.resource::<View>().bottom);
    put_run(&mut w, world.resource::<StageRun>());
    let t = world.resource::<Terrain>();
    w.usize(t.rows).usize(t.tiles.len());
    t.tiles.iter().for_each(|tile| _ = w.u8(index(&Tile::ALL, tile)));
    match world.get_resource::<CityRun>() {
        Some(c) => put_city(w.bool(true), c, slot),
        None => _ = w.bool(false),
    }
    w.usize(ents.len());
    for &(_, e) in &ents {
        let tf = world.get::<Transform>(e).map_or(Vec2::ZERO, |t| t.translation.xy());
        if let (Some(tank), Some(gun)) = (world.get::<Tank>(e), world.get::<Gun>(e)) {
            w.u8(0);
            put_tank(&mut w, tank, gun, world.get::<Enemy>(e));
            for v in [
                world.get::<Spawning>(e).map(|s| s.0),
                world.get::<Shield>(e).map(|s| s.0),
                world.get::<Paralyzed>(e).map(|s| s.0),
                world.get::<HitFlash>(e).map(|s| s.0),
            ] {
                put_opt(&mut w, v);
            }
        } else if let Some(b) = world.get::<Bullet>(e) {
            w.u8(1).u32(slot(b.owner)).u8(team(b.team)).u8(index(&Dir::ALL, &b.dir)).f32(b.speed).bool(b.power);
        } else if let Some(p) = world.get::<PowerUp>(e) {
            w.u8(2).u8(index(&PowerKind::ALL, &p.0)).bool(world.get::<Stash>(e).is_some());
        } else {
            w.u8(3); // the base
        }
        w.vec2(tf);
    }
    w.0
}

/// Loads `bytes` from `take` on a live stage. `None` (nothing changed) if it isn't one, or the bytes don't parse.
pub fn apply(world: &mut World, bytes: &[u8]) -> Option<()> {
    let mut r = R(bytes);
    let screen = r.pick(&SCREENS)?;
    (screen == Screen::Stage && *world.resource::<State<Screen>>().get() == Screen::Stage).then_some(())?;
    let sess = get_session(&mut r)?;
    let (rng, paused, bottom) = (r.u64()?, r.bool()?, r.f32()?);
    let run = get_run(&mut r)?;
    let rows = r.usize()?;
    let tiles = (0..r.usize()?).map(|_| r.pick(&Tile::ALL)).collect::<Option<Vec<_>>>()?;
    let city = if r.bool()? { Some(get_city(&mut r)?) } else { None };
    let things = (0..r.usize()?).map(|_| get_thing(&mut r)).collect::<Option<Vec<_>>>()?;

    // Parsed in full: now replace the world's stage.
    let mut q = world.query_filtered::<Entity, With<NetId>>();
    for e in q.iter(world).collect::<Vec<_>>() {
        world.despawn(e);
    }
    let fresh: Vec<Entity> = things.iter().map(|_| world.spawn_empty().id()).collect();
    let ent = |s: u32| fresh.get(s as usize).copied().unwrap_or(Entity::PLACEHOLDER);
    for (&e, thing) in fresh.iter().zip(things) {
        let mut ec = world.entity_mut(e);
        ec.insert((Transform::from_translation(thing.pos.extend(0.0)), DespawnOnExit(Screen::Stage)));
        match thing.kind {
            Kind::Tank { tank, gun, enemy, timers: [spawning, shield, para, flash] } => {
                ec.insert((tank, gun));
                if let Some(en) = enemy {
                    ec.insert(en);
                }
                if let Some(v) = spawning {
                    ec.insert(Spawning(v));
                }
                if let Some(v) = shield {
                    ec.insert(Shield(v));
                }
                if let Some(v) = para {
                    ec.insert(Paralyzed(v));
                }
                if let Some(v) = flash {
                    ec.insert(HitFlash(v));
                }
            }
            Kind::Bullet { owner, team, dir, speed, power } => _ = ec.insert(Bullet { owner: ent(owner), team, dir, speed, power }),
            Kind::Power { kind, stash: true } => _ = ec.insert((PowerUp(kind), Stash)),
            Kind::Power { kind, stash: false } => _ = ec.insert(PowerUp(kind)),
            Kind::Base => _ = ec.insert(Base),
        }
    }
    let mut session = world.resource_mut::<Session>();
    for (i, new) in sess.slots.into_iter().enumerate() {
        // Devices stay this machine's own (both say `Net(i)` in a session), and so does the local hi score.
        let device = session.slots[i].as_ref().map_or(Device::Net(i as u8), |s| s.device);
        session.slots[i] = new.map(|s| Slot { device, ..s });
    }
    (session.mode, session.stage, session.bonus, session.log) = (sess.mode, sess.stage, sess.bonus, sess.log);
    world.insert_resource(Rng(rng));
    world.resource_mut::<Paused>().0 = paused;
    world.resource_mut::<View>().bottom = bottom;
    world.insert_resource(run);
    let mut terrain = Terrain { tiles, rows, dirty: Vec::new() };
    terrain.dirty = terrain.cells().collect(); // the renderer redraws every cell
    world.insert_resource(terrain);
    match city {
        Some((mut c, turrets, boss)) => {
            c.turrets.iter_mut().zip(turrets).for_each(|(t, s)| t.0 = ent(s));
            c.boss = boss.map(ent);
            world.insert_resource(c);
        }
        None => _ = world.remove_resource::<CityRun>(),
    }
    Some(())
}

fn team(t: Team) -> u8 {
    match t {
        Team::Player(i) => i as u8,
        Team::Enemy => 255,
    }
}

fn get_team(r: &mut R) -> Option<Team> {
    Some(match r.u8()? {
        255 => Team::Enemy,
        i => Team::Player(i as usize),
    })
}

fn put_opt(w: &mut W, v: Option<f32>) {
    match v {
        Some(v) => _ = w.bool(true).f32(v),
        None => _ = w.bool(false),
    }
}

fn get_opt(r: &mut R) -> Option<Option<f32>> {
    Some(if r.bool()? { Some(r.f32()?) } else { None })
}

fn put_session(w: &mut W, s: &Session) {
    for slot in &s.slots {
        match slot {
            None => _ = w.bool(false),
            Some(p) => {
                w.bool(true).u32(p.lives).u32(p.score).u8(p.level).bool(p.out).u32(p.pickups);
                p.kills.iter().chain(&p.big).for_each(|&k| _ = w.u32(k));
            }
        }
    }
    w.u8(index(&MODES, &s.mode)).usize(s.stage).u32(s.bonus).usize(s.log.len());
    for l in &s.log {
        w.usize(l.stage).u32(l.tenths).bool(l.won).usize(l.kills.len());
        for ((k, p), b) in l.kills.iter().zip(&l.pickups).zip(l.big.iter().chain(std::iter::repeat(&[0; 3]))) {
            k.iter().chain(b).for_each(|&v| _ = w.u32(v));
            w.u32(*p);
        }
        w.bool(!l.big.is_empty());
    }
}

/// A parsed session; slot devices are placeholders (each machine keeps its own).
struct SessionData {
    slots: [Option<Slot>; 2],
    mode: Mode,
    stage: usize,
    bonus: u32,
    log: Vec<StageLog>,
}

fn get_session(r: &mut R) -> Option<SessionData> {
    let mut slots = [None, None];
    for s in &mut slots {
        if r.bool()? {
            let mut slot = Slot::new(Device::Net(0));
            (slot.lives, slot.score, slot.level, slot.out, slot.pickups) = (r.u32()?, r.u32()?, r.u8()?, r.bool()?, r.u32()?);
            for k in slot.kills.iter_mut().chain(&mut slot.big) {
                *k = r.u32()?;
            }
            *s = Some(slot);
        }
    }
    let (mode, stage, bonus) = (r.pick(&MODES)?, r.usize()?, r.u32()?);
    let mut log = Vec::new();
    for _ in 0..r.usize()? {
        let (stage, tenths, won, players) = (r.usize()?, r.u32()?, r.bool()?, r.usize()?);
        let mut l = StageLog { stage, tenths, won, kills: vec![], pickups: vec![], big: vec![] };
        for _ in 0..players {
            let v = (0..7).map(|_| r.u32()).collect::<Option<Vec<_>>>()?;
            l.kills.push([v[0], v[1], v[2], v[3]]);
            l.big.push([v[4], v[5], v[6]]);
            l.pickups.push(r.u32()?);
        }
        if !r.bool()? {
            l.big.clear();
        }
        log.push(l);
    }
    Some(SessionData { slots, mode, stage, bonus, log })
}

fn put_run(w: &mut W, s: &StageRun) {
    w.u8(index(&PHASES, &s.phase)).f32(s.timer).usize(s.roster.len());
    s.roster.iter().for_each(|k| _ = w.u8(index(&EnemyKind::ALL, k)));
    w.usize(s.next_enemy).f32(s.spawn_timer).usize(s.spawn_point).f32(s.freeze).f32(s.shovel).bool(s.base_alive).f32(s.clock);
}

fn get_run(r: &mut R) -> Option<StageRun> {
    let (phase, timer) = (r.pick(&PHASES)?, r.f32()?);
    let roster = (0..r.usize()?).map(|_| r.pick(&EnemyKind::ALL)).collect::<Option<Vec<_>>>()?;
    Some(StageRun {
        phase,
        timer,
        roster,
        next_enemy: r.usize()?,
        spawn_timer: r.f32()?,
        spawn_point: r.usize()?,
        freeze: r.f32()?,
        shovel: r.f32()?,
        base_alive: r.bool()?,
        clock: r.f32()?,
    })
}

fn put_bastion(w: &mut W, b: Option<usize>) {
    w.u32(b.map_or(NONE, |b| b as u32));
}

fn get_bastion(r: &mut R) -> Option<Option<usize>> {
    let b = r.u32()?;
    Some((b != NONE).then_some(b as usize))
}

fn put_city(w: &mut W, c: &CityRun, slot: impl Fn(Entity) -> u32) {
    c.mix.iter().for_each(|&m| _ = w.usize(m));
    w.usize(c.pending.len());
    for s in &c.pending {
        w.u8(index(&RANKS, &s.rank)).u8(index(&EnemyKind::ALL, &s.kind)).vec2(s.pos).bool(s.bonus);
        put_bastion(w, s.bastion);
    }
    w.usize(c.turrets.len());
    for &(e, b, pos) in &c.turrets {
        w.u32(slot(e)).vec2(pos);
        put_bastion(w, b);
    }
    w.usize(c.guards.len());
    c.guards.iter().for_each(|&g| _ = w.usize(g));
    w.usize(c.gates.len());
    for gate in &c.gates {
        w.usize(gate.len());
        gate.iter().for_each(|&(x, y)| _ = w.usize(x).usize(y));
    }
    // A dead boss stays `Some` (it stops reinforcements), so presence and slot are separate.
    match c.boss {
        None => _ = w.bool(false),
        Some(b) => _ = w.bool(true).u32(slot(b)),
    }
    w.bool(c.boss_down).f32(c.reinforce_in).usize(c.secrets.len());
    c.secrets.iter().for_each(|(pos, k)| _ = w.vec2(*pos).u8(index(&PowerKind::ALL, k)));
}

/// The district with placeholder entities, plus the snapshot slots of its turrets and boss for `apply` to map.
fn get_city(r: &mut R) -> Option<(CityRun, Vec<u32>, Option<u32>)> {
    let mix = [r.usize()?, r.usize()?, r.usize()?, r.usize()?];
    let pending = (0..r.usize()?)
        .map(|_| {
            let (rank, kind, pos, bonus) = (r.pick(&RANKS)?, r.pick(&EnemyKind::ALL)?, r.vec2()?, r.bool()?);
            Some(Spawn { rank, kind, pos, bonus, bastion: get_bastion(r)? })
        })
        .collect::<Option<Vec<_>>>()?;
    let turrets = (0..r.usize()?).map(|_| Some((r.u32()?, r.vec2()?, get_bastion(r)?))).collect::<Option<Vec<_>>>()?;
    let guards = (0..r.usize()?).map(|_| r.usize()).collect::<Option<Vec<_>>>()?;
    let gates = (0..r.usize()?)
        .map(|_| (0..r.usize()?).map(|_| Some((r.usize()?, r.usize()?))).collect::<Option<Vec<_>>>())
        .collect::<Option<Vec<_>>>()?;
    let boss = if r.bool()? { Some(r.u32()?) } else { None };
    let (boss_down, reinforce_in) = (r.bool()?, r.f32()?);
    let secrets = (0..r.usize()?).map(|_| Some((r.vec2()?, r.pick(&PowerKind::ALL)?))).collect::<Option<Vec<_>>>()?;
    let city = CityRun {
        mix,
        pending,
        turrets: turrets.iter().map(|&(_, pos, b)| (Entity::PLACEHOLDER, b, pos)).collect(),
        guards,
        gates,
        boss: None,
        boss_down,
        reinforce_in,
        secrets,
    };
    Some((city, turrets.iter().map(|t| t.0).collect(), boss))
}

fn put_tank(w: &mut W, t: &Tank, g: &Gun, e: Option<&Enemy>) {
    w.u8(team(t.team)).u8(index(&Dir::ALL, &t.dir)).u8(t.want.map_or(255, |d| index(&Dir::ALL, &d)));
    w.f32(t.speed).bool(t.moving).bool(t.blocked).f32(t.slide).f32(t.odometer).f32(t.size);
    w.f32(g.stats.bullet_speed).usize(g.stats.max_bullets).bool(g.stats.power).bool(g.trigger).f32(g.cooldown);
    match e {
        None => _ = w.bool(false),
        Some(e) => {
            w.bool(true).u8(index(&EnemyKind::ALL, &e.kind)).u8(index(&RANKS, &e.rank));
            w.u32(e.hp).u32(e.max_hp).bool(e.bonus).f32(e.turn_in).f32(e.fire_in);
        }
    }
}

enum Kind {
    Tank { tank: Tank, gun: Gun, enemy: Option<Enemy>, timers: [Option<f32>; 4] },
    Bullet { owner: u32, team: Team, dir: Dir, speed: f32, power: bool },
    Power { kind: PowerKind, stash: bool },
    Base,
}

struct Thing {
    kind: Kind,
    pos: Vec2,
}

fn get_thing(r: &mut R) -> Option<Thing> {
    let kind = match r.u8()? {
        0 => {
            let (team, dir, want) = (get_team(r)?, r.pick(&Dir::ALL)?, r.u8()?);
            let want = if want == 255 { None } else { Some(*Dir::ALL.get(want as usize)?) };
            let mut tank = Tank::new(team, dir, r.f32()?);
            tank.want = want;
            (tank.moving, tank.blocked, tank.slide, tank.odometer, tank.size) = (r.bool()?, r.bool()?, r.f32()?, r.f32()?, r.f32()?);
            let stats = GunStats { bullet_speed: r.f32()?, max_bullets: r.usize()?, power: r.bool()? };
            let gun = Gun { stats, trigger: r.bool()?, cooldown: r.f32()? };
            let enemy = if r.bool()? {
                let (kind, rank) = (r.pick(&EnemyKind::ALL)?, r.pick(&RANKS)?);
                Some(Enemy { kind, rank, hp: r.u32()?, max_hp: r.u32()?, bonus: r.bool()?, turn_in: r.f32()?, fire_in: r.f32()? })
            } else {
                None
            };
            Kind::Tank { tank, gun, enemy, timers: [get_opt(r)?, get_opt(r)?, get_opt(r)?, get_opt(r)?] }
        }
        1 => Kind::Bullet { owner: r.u32()?, team: get_team(r)?, dir: r.pick(&Dir::ALL)?, speed: r.f32()?, power: r.bool()? },
        2 => Kind::Power { kind: r.pick(&PowerKind::ALL)?, stash: r.bool()? },
        3 => Kind::Base,
        _ => return None,
    };
    Some(Thing { kind, pos: r.vec2()? })
}
