//! City Run: a tall district scrolls as the players push up. Placed enemies wake when they come into
//! view, reinforcements drop in from the top, bastion gates open once their turrets fall, a living
//! mini-boss or boss holds the camera, and killing the boss clears the district (unlocks: `progress`).

use super::stage::spawn_enemy;
use super::*;
use crate::city_map::{self, Spawn};
use crate::geom::{FIELD, overlaps, rect_at};
use crate::rng::Rng;
use crate::rules::Rank;
use crate::terrain::{Terrain, Tile};

#[derive(Resource)]
pub struct CityRun {
    pub(crate) mix: [usize; 4],
    /// Not yet in view, bottom first.
    pub(crate) pending: Vec<Spawn>,
    /// Live turrets: entity, bastion, position.
    pub(crate) turrets: Vec<(Entity, Option<usize>, Vec2)>,
    /// Turrets still standing per bastion (pending or live).
    pub(crate) guards: Vec<usize>,
    pub(crate) gates: Vec<Vec<(usize, usize)>>,
    pub(crate) boss: Option<Entity>,
    pub boss_down: bool,
    pub(crate) reinforce_in: f32,
    /// Secret stashes still buried: centre, power-up.
    pub(crate) secrets: Vec<(Vec2, PowerKind)>,
}

/// Cells a tank can drive to from the start (`city_map::reachable`, gates open). Reinforcements drop only there:
/// district 5 had a pocket sealed by steel above a bastion, and a tank dropped into it could never get out.
/// Fixed per district, so it stays out of `CityRun` (and the LAN snapshot).
/// `climb` (`city_map::climbable`) is where a respawned player can get on without backing down out of the view.
#[derive(Resource)]
pub struct Reach {
    cells: Vec<bool>,
    climb: Vec<bool>,
}

impl Reach {
    pub fn new(d: &city_map::District) -> Self {
        Reach { cells: city_map::reachable(d, (8, 0), &[]), climb: city_map::climbable(d) }
    }

    /// A 16-unit drop centred on `p` lands on a reachable cell.
    pub(crate) fn ok(&self, p: Vec2) -> bool {
        Self::at(&self.cells, p)
    }

    /// A 16-unit tank centred on `p` can climb to the map top from there without going below it.
    pub fn climbs(&self, p: Vec2) -> bool {
        Self::at(&self.climb, p)
    }

    fn at(grid: &[bool], p: Vec2) -> bool {
        let (x, y) = (((p.x - 8.0) / 8.0).max(0.0) as usize, ((p.y - 8.0) / 8.0).max(0.0) as usize);
        grid.get(y * crate::geom::CELLS + x) == Some(&true)
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(
        Update,
        (scroll, wake, reinforce, watch, reveal, cull)
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
    commands.insert_resource(Reach::new(&d));
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
        secrets: d.secrets,
    });
    for &(pos, kind) in &d.stashes {
        commands.spawn((PowerUp(kind), Stash, Transform::from_translation(pos.extend(0.0)), DespawnOnExit(Screen::Stage)));
    }
    for slot in session.slots.iter_mut().flatten() {
        // Score is per try: a retry starts from 0, like the clear time the board ranks.
        slot.score = 0;
        slot.lives = rules::CITY_START_LIVES;
        slot.level = 0;
        slot.out = false;
    }
    d.terrain
}

/// Room the view keeps below the trailing tank, so a tank tucked into a notch can still back out of it; the view
/// never scrolls back down, so without it a co-op partner in a pocket was stuck for good (`city_gen::trap`).
pub const TRAIL_ROOM: f32 = 16.0;

/// New view bottom: the lead player sits a bit above the middle, the trailing one stays on screen with
/// `TRAIL_ROOM` below it, and the view never moves back down or past the map top. `locked` (a boss is up) holds it still;
/// it rises at most `max_rise` per frame, so the view glides up after a boss instead of jumping.
pub fn scroll_to(bottom: f32, players_y: &[f32], limit: f32, locked: bool, max_rise: f32) -> f32 {
    let (Some(lead), Some(trail)) = (players_y.iter().copied().reduce(f32::max), players_y.iter().copied().reduce(f32::min)) else {
        return bottom;
    };
    if locked {
        return bottom;
    }
    (lead - FIELD * 0.55).min(trail - 8.0 - TRAIL_ROOM).min(limit).clamp(bottom, bottom + max_rise)
}

fn scroll(mut view: ResMut<View>, time: Res<Time>, terrain: Res<Terrain>, tanks: Query<(&Tank, &Transform)>, enemies: Query<&Enemy>) {
    let ys: Vec<f32> = tanks.iter().filter(|(t, _)| matches!(t.team, Team::Player(_))).map(|(_, tf)| tf.translation.y).collect();
    let locked = enemies.iter().any(|e| matches!(e.rank, Rank::Mini | Rank::Boss));
    let max_rise = rules::VIEW_CATCH_UP * time.delta_secs();
    view.bottom = scroll_to(view.bottom, &ys, terrain.height() - FIELD, locked, max_rise);
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
    reach: Res<Reach>,
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
        // Pending spawns too: a turret's notch in the wall is open ground until it wakes, and a tank dropped
        // there got the turret on top of it, stuck for good.
        reach.ok(p)
            && !terrain.blocks_tank(spot)
            && !tanks.iter().any(|(t, tf)| overlaps(spot, rect_at(tf.translation.xy(), t.size)))
            && !city.pending.iter().any(|s| overlaps(spot, rect_at(s.pos, s.rank.size())))
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

/// A player shell broke into a secret core: the rest of the core crumbles and its power-up pops out (and stays put,
/// like a stash).
fn reveal(
    mut commands: Commands,
    mut city: ResMut<CityRun>,
    mut terrain: ResMut<Terrain>,
    mut fx: MessageWriter<Fx>,
    mut sfx: MessageWriter<Sfx>,
    mut feats: MessageWriter<Feat>,
) {
    city.secrets.retain(|&(pos, kind)| {
        let core: Vec<(usize, usize)> = city_map::footprint(pos, 16.0).collect();
        if core.iter().all(|&(x, y)| terrain.get(x, y) == Tile::Secret) {
            return true;
        }
        for &(x, y) in &core {
            if terrain.get(x, y) == Tile::Secret {
                terrain.set(x, y, Tile::Empty);
            }
        }
        commands.spawn((PowerUp(kind), Stash, Transform::from_translation(pos.extend(0.0)), DespawnOnExit(Screen::Stage)));
        fx.write(Fx { kind: FxKind::Bricks, pos });
        sfx.write(Sfx::PowerAppear);
        feats.write(Feat::Secret);
        false
    });
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn view_follows_the_lead_but_never_drops_the_trailing_tank() {
        assert_eq!(scroll_to(0.0, &[150.0], 1000.0, false, 1000.0), 150.0 - FIELD * 0.55);
        assert_eq!(scroll_to(0.0, &[190.0, 40.0], 1000.0, false, 1000.0), 16.0, "trailing tank pins the bottom, with room");
        assert_eq!(scroll_to(50.0, &[60.0], 1000.0, false, 1000.0), 50.0, "never scrolls back");
        assert_eq!(scroll_to(0.0, &[5000.0], 400.0, false, 1000.0), 400.0, "stops at the map top");
        assert_eq!(scroll_to(10.0, &[300.0], 1000.0, true, 1000.0), 10.0, "boss holds the camera");
        assert_eq!(scroll_to(10.0, &[], 1000.0, false, 1000.0), 10.0);
        assert_eq!(scroll_to(10.0, &[300.0], 1000.0, false, 2.0), 12.0, "after a boss the view glides, no jump");
    }

    #[test]
    fn no_drop_into_a_sealed_pocket() {
        // Owner's report: district 5, left edge just above a bastion, steel all round; a reinforcement dropped
        // there (x 8 is a drop column) and spun for good. Open ground, so only the reach check keeps it out.
        let d = city_map::load(5);
        let reach = Reach::new(&d);
        let pocket = Vec2::new(8.0, 464.0);
        assert!(!d.terrain.blocks_tank(rect_at(pocket, 16.0)));
        assert!(!reach.ok(pocket));
        assert!(reach.ok(Vec2::new(104.0, 8.0)), "the start is reachable");
    }
}
