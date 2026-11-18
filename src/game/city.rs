//! City Run: a tall district scrolls as the players push up. Placed enemies wake when they come into
//! view, reinforcements drop in from the top, bastion gates open once their turrets fall, a living
//! mini-boss or boss holds the camera, and killing the boss clears the district. Unlocks are saved.

use super::stage::spawn_enemy;
use super::*;
use crate::city_map::{self, Spawn};
use crate::geom::{FIELD, overlaps, rect_at};
use crate::rng::Rng;
use crate::rules::{CITY_DISTRICTS, Rank};
use crate::terrain::{Terrain, Tile};

#[derive(Resource)]
pub struct CityRun {
    mix: [usize; 4],
    /// Not yet in view, bottom first.
    pending: Vec<Spawn>,
    /// Live turrets: entity, bastion, position.
    turrets: Vec<(Entity, Option<usize>, Vec2)>,
    /// Turrets still standing per bastion (pending or live).
    guards: Vec<usize>,
    gates: Vec<Vec<(usize, usize)>>,
    boss: Option<Entity>,
    pub boss_down: bool,
    reinforce_in: f32,
}

/// Highest district the players may start (1-based); `CITY_DISTRICTS + 1` once all are cleared.
#[derive(Resource, Clone, Copy, PartialEq, Debug)]
pub struct Progress {
    pub unlocked: usize,
}

impl Progress {
    pub fn parse(text: &str) -> Self {
        let n = text.lines().find_map(|l| l.trim().strip_prefix("unlocked=")).and_then(|v| v.trim().parse().ok()).unwrap_or(1);
        Progress { unlocked: usize::clamp(n, 1, CITY_DISTRICTS + 1) }
    }
    /// District `n` cleared: open the next one. True if that is news.
    pub fn clear(&mut self, n: usize) -> bool {
        let next = (n + 1).min(CITY_DISTRICTS + 1);
        let news = next > self.unlocked;
        self.unlocked = self.unlocked.max(next);
        news
    }
}

const SAVE: &str = "city.txt";

pub fn plugin(app: &mut App) {
    let progress = crate::settings::data_file(SAVE)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map_or(Progress { unlocked: 1 }, |t| Progress::parse(&t));
    app.insert_resource(progress)
        .add_systems(OnEnter(Screen::Results), save_clear.run_if(|s: Res<Session>| s.mode == Mode::City))
        .add_systems(
            Update,
            (scroll, wake, reinforce, watch, cull)
                .chain()
                .in_set(Sim)
                .after(stage::tick_timers)
                .before(tank::player_control)
                .run_if(resource_exists::<CityRun>),
        );
}

/// Builds district `session.stage`: resets lives and power (every district starts fresh), fills the stashes,
/// returns its terrain.
pub fn setup(commands: &mut Commands, session: &mut Session) -> Terrain {
    let d = city_map::load(session.stage);
    let mut guards = vec![0; d.gates.len()];
    for b in d.spawns.iter().filter_map(|s| s.bastion) {
        guards[b] += 1;
    }
    commands.insert_resource(CityRun {
        mix: d.mix,
        pending: d.spawns,
        turrets: Vec::new(),
        guards,
        gates: d.gates,
        boss: None,
        boss_down: false,
        reinforce_in: rules::reinforce_interval(session.stage, session.players()),
    });
    for &(pos, kind) in &d.stashes {
        commands.spawn((PowerUp(kind), Stash, Transform::from_translation(pos.extend(0.0)), DespawnOnExit(Screen::Stage)));
    }
    for slot in session.slots.iter_mut().flatten() {
        slot.lives = rules::CITY_START_LIVES;
        slot.level = 0;
        slot.out = false;
    }
    d.terrain
}

/// New view bottom: the lead player sits a bit above the middle, the trailing one stays fully on screen,
/// and the view never moves back down or past the map top. `locked` (a boss is up) holds it still.
pub fn scroll_to(bottom: f32, players_y: &[f32], limit: f32, locked: bool) -> f32 {
    let (Some(lead), Some(trail)) = (players_y.iter().copied().reduce(f32::max), players_y.iter().copied().reduce(f32::min)) else {
        return bottom;
    };
    if locked {
        return bottom;
    }
    (lead - FIELD * 0.55).min(trail - 8.0).min(limit).max(bottom)
}

fn scroll(mut view: ResMut<View>, terrain: Res<Terrain>, tanks: Query<(&Tank, &Transform)>, enemies: Query<&Enemy>) {
    let ys: Vec<f32> = tanks.iter().filter(|(t, _)| matches!(t.team, Team::Player(_))).map(|(_, tf)| tf.translation.y).collect();
    let locked = enemies.iter().any(|e| matches!(e.rank, Rank::Mini | Rank::Boss));
    view.bottom = scroll_to(view.bottom, &ys, terrain.height() - FIELD, locked);
}

fn wake(mut commands: Commands, mut city: ResMut<CityRun>, view: Res<View>, session: Res<Session>, mut rng: ResMut<Rng>) {
    while city.pending.first().is_some_and(|s| s.pos.y + s.rank.size() / 2.0 <= view.top()) {
        let s = city.pending.remove(0);
        let e = spawn_enemy(&mut commands, s.kind, s.rank, s.pos, s.bonus, session.stage, &mut rng);
        match s.rank {
            Rank::Turret => city.turrets.push((e, s.bastion, s.pos)),
            Rank::Boss => city.boss = Some(e),
            _ => {}
        }
    }
}

/// Fresh enemies from the top edge, capped like a classic stage; none once the boss is up.
fn reinforce(
    mut commands: Commands,
    time: Res<Time>,
    mut city: ResMut<CityRun>,
    view: Res<View>,
    run: Res<StageRun>,
    session: Res<Session>,
    terrain: Res<Terrain>,
    tanks: Query<(&Tank, &Transform)>,
    enemies: Query<&Enemy>,
    mut rng: ResMut<Rng>,
) {
    if run.phase != Phase::Play || city.boss.is_some() {
        return;
    }
    city.reinforce_in -= time.delta_secs();
    let normal = enemies.iter().filter(|e| e.rank == Rank::Normal).count();
    if city.reinforce_in > 0.0 || normal >= rules::max_enemies(session.players()) {
        return;
    }
    let y = (view.top() / 8.0).floor() * 8.0 - 8.0;
    let start = rng.below(5) as usize;
    let free = (0..5).map(|i| [104.0, 8.0, 200.0, 56.0, 152.0][(start + i) % 5]).map(|x| Vec2::new(x, y)).find(|&p| {
        let spot = rect_at(p, 16.0);
        !terrain.blocks_tank(spot) && !tanks.iter().any(|(t, tf)| overlaps(spot, rect_at(tf.translation.xy(), t.size)))
    });
    if let Some(p) = free {
        let kind = city_map::weighted(&city.mix, &mut rng);
        spawn_enemy(&mut commands, kind, Rank::Normal, p, false, session.stage, &mut rng);
        city.reinforce_in = rules::reinforce_interval(session.stage, session.players());
    }
}

/// Dead turrets wall up their spot; a bastion with no turrets left opens its gate. Notes the boss's death.
fn watch(
    mut city: ResMut<CityRun>,
    mut terrain: ResMut<Terrain>,
    alive: Query<(), With<Enemy>>,
    mut fx: MessageWriter<Fx>,
    mut sfx: MessageWriter<Sfx>,
) {
    let CityRun { turrets, guards, gates, .. } = &mut *city;
    let mut opened = Vec::new();
    turrets.retain(|&(e, bastion, pos)| {
        if alive.contains(e) {
            return true;
        }
        city_map::footprint(pos, 16.0).for_each(|(x, y)| terrain.set(x, y, Tile::Wall));
        if let Some(b) = bastion {
            guards[b] -= 1;
            if guards[b] == 0 {
                opened.push(b);
            }
        }
        false
    });
    for b in opened {
        for &(x, y) in &gates[b] {
            terrain.set(x, y, Tile::Empty);
            if (x + y) % 4 == 0 {
                fx.write(Fx { kind: FxKind::Sparks, pos: Vec2::new(x as f32 * 4.0 + 2.0, y as f32 * 4.0 + 2.0) });
            }
        }
        sfx.write(Sfx::GateOpen);
    }
    if city.boss.is_some_and(|b| !alive.contains(b)) {
        city.boss_down = true;
    }
}

/// Enemies and power-ups the view has scrolled past are gone (no points).
fn cull(mut commands: Commands, view: Res<View>, things: Query<(Entity, &Transform, Option<&Tank>), Or<(With<Enemy>, With<PowerUp>)>>) {
    for (e, tf, tank) in &things {
        let half = tank.map_or(8.0, |t| t.size / 2.0);
        if tf.translation.y + half < view.bottom + 4.0 {
            commands.entity(e).try_despawn();
        }
    }
}

fn save_clear(session: Res<Session>, mut progress: ResMut<Progress>) {
    if progress.clear(session.stage)
        && let Some(p) = crate::settings::data_file(SAVE)
    {
        let result = p
            .parent()
            .map_or(Ok(()), std::fs::create_dir_all)
            .and_then(|_| std::fs::write(&p, format!("unlocked={}\n", progress.unlocked)));
        if let Err(e) = result {
            warn!("could not save City Run progress to {}: {e}", p.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_follows_the_lead_but_never_drops_the_trailing_tank() {
        assert_eq!(scroll_to(0.0, &[150.0], 1000.0, false), 150.0 - FIELD * 0.55);
        assert_eq!(scroll_to(0.0, &[190.0, 20.0], 1000.0, false), 12.0, "trailing tank pins the bottom");
        assert_eq!(scroll_to(50.0, &[60.0], 1000.0, false), 50.0, "never scrolls back");
        assert_eq!(scroll_to(0.0, &[5000.0], 400.0, false), 400.0, "stops at the map top");
        assert_eq!(scroll_to(10.0, &[300.0], 1000.0, true), 10.0, "boss holds the camera");
        assert_eq!(scroll_to(10.0, &[], 1000.0, false), 10.0);
    }

    #[test]
    fn progress_parses_and_only_moves_forward() {
        assert_eq!(Progress::parse("").unlocked, 1);
        assert_eq!(Progress::parse("unlocked=4\n").unlocked, 4);
        assert_eq!(Progress::parse("unlocked=99").unlocked, CITY_DISTRICTS + 1);
        let mut p = Progress { unlocked: 3 };
        assert!(!p.clear(1), "replaying an old district changes nothing");
        assert!(p.clear(3));
        assert_eq!(p.unlocked, 4);
        p.clear(CITY_DISTRICTS);
        assert_eq!(p.unlocked, CITY_DISTRICTS + 1);
    }
}
