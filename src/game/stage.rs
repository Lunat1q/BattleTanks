//! Stage lifecycle: build on enter, enemy spawning, timers, win/lose, pause and controller loss.

use super::*;
use crate::geom::{FIELD, overlaps, rect_at};
use crate::input::{Device, DeviceIntents};
use crate::levels;
use crate::rng::Rng;
use crate::rules::{self, BONUS_CARRIERS};
use crate::terrain::{Terrain, Tile};

pub const PLAYER_SPAWNS: [Vec2; 2] = [Vec2::new(72.0, 8.0), Vec2::new(136.0, 8.0)];
pub const ENEMY_SPAWNS: [Vec2; 3] = [Vec2::new(104.0, FIELD - 8.0), Vec2::new(FIELD - 8.0, FIELD - 8.0), Vec2::new(8.0, FIELD - 8.0)];
pub const BASE_POS: Vec2 = Vec2::new(104.0, 8.0);
const INTRO_TIME: f32 = 2.2;

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Stage), setup)
        .add_systems(OnExit(Screen::Stage), |mut paused: ResMut<Paused>| paused.0 = false)
        .add_systems(Update, (watch_controllers, toggle_pause, advance_phase).chain().run_if(in_state(Screen::Stage)));
}

fn setup(mut commands: Commands, mut session: ResMut<Session>, mut sfx: MessageWriter<Sfx>) {
    let stage = levels::load(session.stage);
    let mut terrain = stage.terrain;
    // Everything is dirty once so the renderer builds the whole map.
    terrain.dirty = (0..crate::geom::SUBS).flat_map(|y| (0..crate::geom::SUBS).map(move |x| (x, y))).collect();
    commands.insert_resource(terrain);
    commands.insert_resource(StageRun {
        phase: Phase::Intro,
        timer: INTRO_TIME,
        roster: stage.roster,
        next_enemy: 0,
        spawn_timer: 0.0,
        spawn_point: 0,
        freeze: 0.0,
        shovel: 0.0,
        base_alive: true,
    });
    commands.spawn((Base, Transform::from_translation(BASE_POS.extend(0.0)), DespawnOnExit(Screen::Stage)));
    for i in 0..2 {
        if let Some(slot) = session.slots[i].as_mut() {
            slot.kills = [0; 4];
            if !slot.out {
                spawn_player(&mut commands, i, slot.level);
            }
        }
    }
    sfx.write(Sfx::StageStart);
}

pub fn spawn_player(commands: &mut Commands, slot: usize, level: u8) {
    commands.spawn((
        Tank::new(Team::Player(slot), Dir::Up, rules::PLAYER_SPEED),
        Gun { stats: rules::player_gun(level), trigger: false, cooldown: 0.0 },
        Spawning(rules::SPAWN_TIME),
        Transform::from_translation(PLAYER_SPAWNS[slot].extend(0.0)),
        DespawnOnExit(Screen::Stage),
    ));
}

pub fn spawn_enemies(
    mut commands: Commands,
    mut run: ResMut<StageRun>,
    session: Res<Session>,
    time: Res<Time>,
    tanks: Query<&Transform, With<Tank>>,
    enemies: Query<(), With<Enemy>>,
    mut rng: ResMut<Rng>,
) {
    let players = session.players();
    let enemies_alive = enemies.iter().count();
    if run.next_enemy >= run.roster.len() || run.phase != Phase::Play {
        return;
    }
    run.spawn_timer -= time.delta_secs();
    if run.spawn_timer > 0.0 || enemies_alive >= rules::max_enemies(players) {
        return;
    }
    let pos = ENEMY_SPAWNS[run.spawn_point % 3];
    let spot = rect_at(pos, 16.0);
    if tanks.iter().any(|t| overlaps(spot, rect_at(t.translation.xy(), 16.0))) {
        run.spawn_point += 1; // occupied, try the next point next frame
        return;
    }
    let kind = run.roster[run.next_enemy];
    commands.spawn((
        Tank::new(Team::Enemy, Dir::Down, kind.speed()),
        Gun { stats: GunStats { bullet_speed: kind.bullet_speed(), max_bullets: 1, power: false }, trigger: false, cooldown: 0.0 },
        Enemy {
            kind,
            hp: kind.hp(),
            bonus: BONUS_CARRIERS.contains(&run.next_enemy),
            turn_in: rng.range(0.5, 2.0),
            fire_in: rng.range(0.5, 1.5),
        },
        Spawning(rules::SPAWN_TIME),
        Transform::from_translation(pos.extend(0.0)),
        DespawnOnExit(Screen::Stage),
    ));
    run.next_enemy += 1;
    run.spawn_point += 1;
    run.spawn_timer = rules::spawn_interval(session.stage, players);
}

pub fn tick_timers(
    mut commands: Commands,
    time: Res<Time>,
    mut run: ResMut<StageRun>,
    mut terrain: ResMut<Terrain>,
    mut timed: Query<(Entity, &Tank, &mut Gun, Option<&mut Spawning>, Option<&mut Shield>, Option<&mut Paralyzed>, Option<&mut HitFlash>)>,
) {
    let dt = time.delta_secs();
    for (e, tank, mut gun, spawning, shield, para, flash) in &mut timed {
        gun.cooldown -= dt;
        if let Some(mut s) = spawning {
            s.0 -= dt;
            if s.0 <= 0.0 {
                commands.entity(e).remove::<Spawning>();
                if matches!(tank.team, Team::Player(_)) {
                    commands.entity(e).insert(Shield(rules::RESPAWN_SHIELD));
                }
            }
        }
        if let Some(mut s) = shield {
            s.0 -= dt;
            if s.0 <= 0.0 {
                commands.entity(e).remove::<Shield>();
            }
        }
        if let Some(mut p) = para {
            p.0 -= dt;
            if p.0 <= 0.0 {
                commands.entity(e).remove::<Paralyzed>();
            }
        }
        if let Some(mut f) = flash {
            f.0 -= dt;
            if f.0 <= 0.0 {
                commands.entity(e).remove::<HitFlash>();
            }
        }
    }
    run.freeze = (run.freeze - dt).max(0.0);
    if run.shovel > 0.0 {
        run.shovel = (run.shovel - dt).max(0.0);
        // Steel, blinking back to brick during the last 3 seconds.
        let steel = run.shovel > 3.0 || ((run.shovel * 4.0) as u32).is_multiple_of(2);
        terrain.set_base_ring(if steel && run.shovel > 0.0 { Tile::Steel } else { Tile::Brick });
    }
}

pub fn check_end(mut run: ResMut<StageRun>, session: Res<Session>, enemies: Query<(), With<Enemy>>, mut sfx: MessageWriter<Sfx>) {
    if run.phase != Phase::Play {
        return;
    }
    let everyone_out = session.slots.iter().flatten().all(|s| s.out);
    if !run.base_alive || everyone_out {
        run.phase = Phase::Lost;
        run.timer = rules::END_DELAY;
        sfx.write(Sfx::GameOver);
    } else if run.next_enemy >= run.roster.len() && enemies.is_empty() {
        run.phase = Phase::Won;
        run.timer = rules::END_DELAY;
    }
}

fn advance_phase(time: Res<Time>, paused: Res<Paused>, mut run: ResMut<StageRun>, mut next: ResMut<NextState<Screen>>) {
    if paused.0 || run.phase == Phase::Play {
        return;
    }
    run.timer -= time.delta_secs();
    if run.timer > 0.0 {
        return;
    }
    match run.phase {
        Phase::Intro => run.phase = Phase::Play,
        Phase::Won => next.set(Screen::Results),
        Phase::Lost => next.set(Screen::GameOver),
        Phase::Play => {}
    }
}

fn toggle_pause(
    session: Res<Session>,
    devices: Res<DeviceIntents>,
    run: Res<StageRun>,
    mut paused: ResMut<Paused>,
    mut sfx: MessageWriter<Sfx>,
) {
    let start = (0..2).any(|i| session.intent(i, &devices).start);
    let lost = session.slots.iter().flatten().any(|s| !devices.connected(s.device));
    if start && !lost && matches!(run.phase, Phase::Play | Phase::Won) {
        paused.0 = !paused.0;
        sfx.write(Sfx::Pause);
    }
}

/// A slot whose controller vanished pauses the game; any free controller pressing A takes it over.
fn watch_controllers(mut session: ResMut<Session>, devices: Res<DeviceIntents>, mut paused: ResMut<Paused>) {
    let used: Vec<Device> = session.slots.iter().flatten().map(|s| s.device).collect();
    let free_press =
        devices.0.iter().find(|(d, i)| matches!(d, Device::Pad(_)) && (i.confirm || i.start) && !used.contains(d)).map(|(d, _)| *d);
    for slot in session.slots.iter_mut().flatten() {
        if !devices.connected(slot.device) {
            paused.0 = true;
            if let Some(d) = free_press {
                slot.device = d;
                return;
            }
        }
    }
}
