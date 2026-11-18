//! Stage lifecycle: build on enter, enemy spawning, timers, win/lose, pause and controller loss.

use super::*;
use crate::geom::{FIELD, overlaps, rect_at};
use crate::input::{Device, DeviceIntents};
use crate::levels;
use crate::rng::Rng;
use crate::rules::{self, BONUS_CARRIERS, Rank};
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

fn setup(mut commands: Commands, mut session: ResMut<Session>, mut view: ResMut<View>, mut sfx: MessageWriter<Sfx>) {
    view.bottom = 0.0;
    let (mut terrain, roster) = if session.mode == Mode::City {
        (super::city::setup(&mut commands, &mut session), Vec::new())
    } else {
        commands.remove_resource::<super::city::CityRun>();
        let stage = levels::load(session.stage);
        commands.spawn((Base, Transform::from_translation(BASE_POS.extend(0.0)), DespawnOnExit(Screen::Stage)));
        (stage.terrain, stage.roster)
    };
    // Everything is dirty once so the renderer builds the whole map.
    terrain.dirty = terrain.cells().collect();
    commands.insert_resource(terrain);
    commands.insert_resource(StageRun {
        phase: Phase::Intro,
        timer: INTRO_TIME,
        roster,
        next_enemy: 0,
        spawn_timer: 0.0,
        spawn_point: 0,
        freeze: 0.0,
        shovel: 0.0,
        base_alive: true,
    });
    for (i, slot) in session.slots.iter_mut().enumerate() {
        if let Some(slot) = slot.as_mut() {
            slot.kills = [0; 4];
            if !slot.out {
                spawn_player(&mut commands, i, slot.level, PLAYER_SPAWNS[i]);
            }
        }
    }
    sfx.write(Sfx::StageStart);
}

pub fn spawn_player(commands: &mut Commands, slot: usize, level: u8, pos: Vec2) {
    commands.spawn((
        Tank::new(Team::Player(slot), Dir::Up, rules::PLAYER_SPEED),
        Gun { stats: rules::player_gun(level), trigger: false, cooldown: 0.0 },
        Spawning(rules::SPAWN_TIME),
        Transform::from_translation(pos.extend(0.0)),
        DespawnOnExit(Screen::Stage),
    ));
}

/// Where player `slot` re-enters: its start column at the bottom of the view, or the nearest open
/// tank-sized spot beside or above it. On a classic stage that is always the start itself.
pub fn respawn_point(slot: usize, bottom: f32, terrain: &Terrain) -> Vec2 {
    let x0 = PLAYER_SPAWNS[slot].x;
    let y0 = (bottom / 8.0).ceil() * 8.0 + 8.0;
    (0..12)
        .flat_map(|row| [0.0, -16.0, 16.0, -32.0, 32.0, -48.0, 48.0].map(|dx| Vec2::new(x0 + dx, y0 + row as f32 * 8.0)))
        .find(|&p| !terrain.blocks_tank(rect_at(p, 16.0)))
        .unwrap_or(Vec2::new(x0, y0))
}

/// One enemy tank; `district` scales City Run ranks (ignored for `Normal`).
pub fn spawn_enemy(commands: &mut Commands, kind: EnemyKind, rank: Rank, pos: Vec2, bonus: bool, district: usize, rng: &mut Rng) -> Entity {
    let hp = rank.hp(kind, district);
    commands
        .spawn((
            Tank { size: rank.size(), ..Tank::new(Team::Enemy, Dir::Down, rank.speed(kind)) },
            Gun {
                stats: GunStats { bullet_speed: kind.bullet_speed(), max_bullets: rank.max_bullets(), power: false },
                trigger: false,
                cooldown: 0.0,
            },
            Enemy { kind, rank, hp, max_hp: hp, bonus, turn_in: rng.range(0.5, 2.0), fire_in: rng.range(0.5, 1.5) },
            Spawning(rules::SPAWN_TIME),
            Transform::from_translation(pos.extend(0.0)),
            DespawnOnExit(Screen::Stage),
        ))
        .id()
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
    spawn_enemy(&mut commands, kind, Rank::Normal, pos, BONUS_CARRIERS.contains(&run.next_enemy), session.stage, &mut rng);
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

pub fn check_end(
    mut run: ResMut<StageRun>,
    session: Res<Session>,
    city: Option<Res<super::city::CityRun>>,
    enemies: Query<(), With<Enemy>>,
    mut sfx: MessageWriter<Sfx>,
) {
    if run.phase != Phase::Play {
        return;
    }
    let everyone_out = session.slots.iter().flatten().all(|s| s.out);
    if !run.base_alive || everyone_out {
        run.phase = Phase::Lost;
        run.timer = rules::END_DELAY;
        sfx.write(Sfx::GameOver);
    } else if city.map_or(run.next_enemy >= run.roster.len() && enemies.is_empty(), |c| c.boss_down) {
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
    let start = (0..2).any(|i| session.intent(i, &devices).start || session.intent(i, &devices).pause);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn respawn_finds_open_ground_at_the_bottom_of_the_view() {
        let classic = Terrain::default();
        assert_eq!(respawn_point(0, 0.0, &classic), PLAYER_SPAWNS[0]);
        let mut ter = Terrain::new(crate::geom::SUBS * 3);
        for y in 75..79 {
            for x in 16..20 {
                ter.set(x, y, Tile::Steel); // right where player 1 would land at bottom = 293
            }
        }
        let p = respawn_point(0, 293.0, &ter);
        assert!(p.y >= 304.0 && !ter.blocks_tank(rect_at(p, 16.0)), "{p}");
    }
}
