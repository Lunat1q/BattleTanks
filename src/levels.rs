//! Stage files (`levels/NN.txt`, baked into the exe) -> terrain + enemy roster. Pure, no ECS.
//!
//! Format:
//! ```text
//! ; comment
//! enemies 18 2 0 0      <- basic fast power armor (sums to 20)
//! .............          <- 26 rows; 13 chars = left half, mirrored; 26 chars = full row
//! ```
//! Cells (8x8): `.` empty, `#` brick, `@` steel, `~` water, `%` trees, `-` ice.
//! Spawn points, the base and its brick ring are forced by the loader, so maps can't block them.

use crate::geom::{CELLS, FIELD};
use crate::rng::Rng;
use crate::rules::{ENEMIES_PER_STAGE, EnemyKind};
use crate::terrain::{Terrain, Tile};
use bevy::math::Rect;

const STAGES: &[&str] = &[
    include_str!("../levels/01.txt"),
    include_str!("../levels/02.txt"),
    include_str!("../levels/03.txt"),
    include_str!("../levels/04.txt"),
    include_str!("../levels/05.txt"),
    include_str!("../levels/06.txt"),
    include_str!("../levels/07.txt"),
    include_str!("../levels/08.txt"),
    include_str!("../levels/09.txt"),
    include_str!("../levels/10.txt"),
    include_str!("../levels/11.txt"),
    include_str!("../levels/12.txt"),
    include_str!("../levels/13.txt"),
    include_str!("../levels/14.txt"),
    include_str!("../levels/15.txt"),
    include_str!("../levels/16.txt"),
    include_str!("../levels/17.txt"),
    include_str!("../levels/18.txt"),
    include_str!("../levels/19.txt"),
    include_str!("../levels/20.txt"),
    include_str!("../levels/21.txt"),
    include_str!("../levels/22.txt"),
    include_str!("../levels/23.txt"),
    include_str!("../levels/24.txt"),
    include_str!("../levels/25.txt"),
    include_str!("../levels/26.txt"),
    include_str!("../levels/27.txt"),
    include_str!("../levels/28.txt"),
    include_str!("../levels/29.txt"),
    include_str!("../levels/30.txt"),
];

pub struct Stage {
    pub terrain: Terrain,
    /// Spawn order for this stage.
    pub roster: Vec<EnemyKind>,
}

/// Areas kept clear: three enemy spawns, two player spawns.
pub fn spawn_areas() -> [Rect; 5] {
    let top = FIELD - 16.0;
    [
        Rect::new(0.0, top, 16.0, FIELD),
        Rect::new(96.0, top, 112.0, FIELD),
        Rect::new(192.0, top, 208.0, FIELD),
        Rect::new(64.0, 0.0, 80.0, 16.0),
        Rect::new(128.0, 0.0, 144.0, 16.0),
    ]
}

pub fn parse(src: &str) -> Result<(Terrain, [usize; 4]), String> {
    let mut counts = None;
    let mut rows = Vec::new();
    for line in src.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with(';')) {
        if let Some(rest) = line.strip_prefix("enemies") {
            let n: Vec<usize> =
                rest.split_whitespace().map(|s| s.parse().map_err(|_| format!("bad count {s:?}"))).collect::<Result<_, _>>()?;
            let n: [usize; 4] = n.try_into().map_err(|_| "enemies needs 4 counts".to_string())?;
            if n.iter().sum::<usize>() != ENEMIES_PER_STAGE {
                return Err(format!("enemies must sum to {ENEMIES_PER_STAGE}"));
            }
            counts = Some(n);
            continue;
        }
        rows.push(row(line, rows.len())?);
    }
    if rows.len() != CELLS {
        return Err(format!("{} rows, want {CELLS}", rows.len()));
    }
    let mut ter = paint(&rows)?;
    for a in spawn_areas().iter().chain([&Rect::new(96.0, 0.0, 112.0, 16.0)]) {
        for y in (a.min.y / 4.0) as usize..(a.max.y / 4.0) as usize {
            for x in (a.min.x / 4.0) as usize..(a.max.x / 4.0) as usize {
                ter.set(x, y, Tile::Empty);
            }
        }
    }
    ter.set_base_ring(Tile::Brick);
    Ok((ter, counts.ok_or("missing `enemies` line")?))
}

/// Map line number `n` (0-based) as 26 cells: written out, or 13 mirrored.
pub fn row(line: &str, n: usize) -> Result<Vec<char>, String> {
    let chars: Vec<char> = line.chars().collect();
    match chars.len() {
        13 => Ok(chars.iter().chain(chars.iter().rev()).copied().collect()),
        26 => Ok(chars),
        len => Err(format!("row {} has {len} cells, want 13 or 26", n + 1)),
    }
}

/// 26 rows of 26 cells (row 0 is the top) as a classic-sized terrain.
pub fn paint(rows: &[Vec<char>]) -> Result<Terrain, String> {
    let mut ter = Terrain::default();
    for (r, row) in rows.iter().enumerate() {
        for (c, ch) in row.iter().enumerate() {
            let tile = match ch {
                '.' => Tile::Empty,
                '#' => Tile::Brick,
                '@' => Tile::Steel,
                '~' => Tile::Water,
                '%' => Tile::Trees,
                '-' => Tile::Ice,
                other => return Err(format!("unknown cell {other:?} at row {}", r + 1)),
            };
            let (x, y) = (c * 2, (CELLS - 1 - r) * 2);
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                ter.set(x + dx, y + dy, tile);
            }
        }
    }
    Ok(ter)
}

/// Steel and water: what a tank can never get through (bricks can be shot away).
pub fn hard_wall(_: usize, _: usize, t: Tile) -> bool {
    matches!(t, Tile::Steel | Tile::Water)
}

/// A 16x16 tank fits with its bottom-left at 8-unit cell `(x, y)`; `wall(cx, cy, tile)` says which cells it can't enter.
pub fn fits(ter: &Terrain, x: usize, y: usize, wall: &impl Fn(usize, usize, Tile) -> bool) -> bool {
    let open = |cx: usize, cy: usize| (0..2).all(|dx| (0..2).all(|dy| !wall(cx, cy, ter.get(cx * 2 + dx, cy * 2 + dy))));
    x < CELLS - 1 && y < CELLS - 1 && (0..2).all(|dx| (0..2).all(|dy| open(x + dx, y + dy)))
}

/// The four cells next to `(x, y)` (fewer at the field's low edges).
pub fn neighbours(x: usize, y: usize) -> impl Iterator<Item = (usize, usize)> {
    [(x + 1, y), (x, y + 1)].into_iter().chain(x.checked_sub(1).map(|x| (x, y))).chain(y.checked_sub(1).map(|y| (x, y)))
}

/// Tank-sized breadth-first fill over cells from `from`: steps to each cell (`y * CELLS + x`), `None` where it can't get.
pub fn steps(ter: &Terrain, from: (usize, usize), wall: impl Fn(usize, usize, Tile) -> bool) -> Vec<Option<u32>> {
    let mut dist = vec![None; CELLS * CELLS];
    if !fits(ter, from.0, from.1, &wall) {
        return dist;
    }
    dist[from.1 * CELLS + from.0] = Some(0);
    let mut todo = std::collections::VecDeque::from([from]);
    while let Some((x, y)) = todo.pop_front() {
        let d = dist[y * CELLS + x].map(|d| d + 1);
        for (nx, ny) in neighbours(x, y) {
            if dist[ny * CELLS + nx].is_none() && fits(ter, nx, ny, &wall) {
                dist[ny * CELLS + nx] = d;
                todo.push_back((nx, ny));
            }
        }
    }
    dist
}

/// Stage `n`'s file (1-based).
pub fn source(n: usize) -> &'static str {
    STAGES[(n.max(1) - 1) % STAGES.len()]
}

pub fn stage_count() -> usize {
    STAGES.len()
}

/// Stage `n` (1-based). Past the last map the maps loop with tougher rosters.
pub fn load(n: usize) -> Stage {
    let idx = (n.max(1) - 1) % STAGES.len();
    let lap = (n.max(1) - 1) / STAGES.len();
    let (mut terrain, mut counts) = parse(STAGES[idx]).unwrap_or_else(|e| panic!("levels/{:02}.txt: {e}", idx + 1));
    terrain.dirty.clear();
    for i in 0..lap * 4 {
        // Each lap turns four weaker tanks into stronger ones.
        if let Some(from) = (0..3).find(|&k| counts[k] > 0) {
            counts[from] -= 1;
            counts[(from + 1 + i % 3).min(3)] += 1;
        }
    }
    let mut roster: Vec<EnemyKind> = EnemyKind::ALL.iter().zip(counts).flat_map(|(&k, c)| std::iter::repeat_n(k, c)).collect();
    // Deterministic per stage, so a stage always plays the same.
    Rng::new(n as u64 * 7919).shuffle(&mut roster);
    // The opening wave (one per spawn point) is the stage's weakest tanks, so nothing fast rushes in before
    // players have moved.
    for i in 0..3 {
        let j = (i..roster.len()).min_by_key(|&j| roster[j].index()).unwrap();
        roster.swap(i, j);
    }
    Stage { terrain, roster }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::CELLS;

    fn reachable(ter: &Terrain, from: (usize, usize)) -> Vec<bool> {
        steps(ter, from, hard_wall).iter().map(Option::is_some).collect()
    }

    fn stages() -> impl Iterator<Item = (String, Terrain)> {
        STAGES.iter().enumerate().map(|(i, src)| (format!("levels/{:02}.txt", i + 1), parse(src).unwrap().0))
    }

    /// Enemy spawn cells (top-left, centre, top-right) and player spawn cells.
    const TOP: [(usize, usize); 3] = [(0, 24), (12, 24), (24, 24)];
    const PLAYERS: [(usize, usize); 2] = [(8, 0), (16, 0)];

    #[test]
    fn every_spawn_can_reach_the_base() {
        // Just above the base ring's top edge, centred on the base.
        let target = 2 * CELLS + 12;
        for (i, src) in STAGES.iter().enumerate() {
            let (ter, _) = parse(src).unwrap();
            for spawn in [(0, 24), (12, 24), (24, 24), (8, 0), (16, 0)] {
                assert!(reachable(&ter, spawn)[target], "levels/{:02}.txt: spawn {spawn:?} is walled off from the base", i + 1);
            }
        }
    }

    /// Every column over the base ring (sub-cells 22..30) has steel between the top spawns and the ring, so no
    /// enemy can shoot the base straight down the centre lane; the base is open only from the sides.
    fn ring_roofed_with_steel(ter: &Terrain) -> bool {
        (22..30).all(|x| (6..48).any(|y| ter.get(x, y) == Tile::Steel))
    }

    #[test]
    fn centre_spawn_has_no_free_shot_at_the_base() {
        // Regression: stage 1 had an open column from the centre spawn, so an enemy killed the base in 3 shots;
        // later most stages had only 2 brick cells in the lane, so the base needed a full-time guard.
        let open: Vec<String> = STAGES
            .iter()
            .enumerate()
            .filter(|(_, src)| !ring_roofed_with_steel(&parse(src).unwrap().0))
            .map(|(i, _)| format!("levels/{:02}.txt", i + 1))
            .collect();
        assert!(open.is_empty(), "no steel roof over the base ring: {open:?}");
    }

    #[test]
    fn no_quick_run_from_a_top_spawn_to_the_backline() {
        // Regression (owner): stages 5, 7, 21, 24 had open lanes down the sides, others a zigzag of 2-4 sidesteps,
        // so in 1P a scout reached the base's row before the player could turn round. Without shooting a brick,
        // the way from a top spawn to the bottom rows (tank at cell row 2 or lower) is at least MIN_RUN cells;
        // straight down is 22.
        const MIN_RUN: u32 = 30;
        let no_bricks = |cx, cy, t| hard_wall(cx, cy, t) || t == Tile::Brick;
        for (name, ter) in stages() {
            for spawn in TOP {
                let d = steps(&ter, spawn, no_bricks);
                let run = d[..3 * CELLS].iter().flatten().min();
                assert!(run.is_none_or(|&r| r >= MIN_RUN), "{name}: brick-free run of {run:?} cells from {spawn:?} to the backline");
            }
        }
    }

    #[test]
    fn a_tank_in_the_centre_lane_cannot_seal_the_field() {
        // Regression (owner): stage 17's steel pickets left one gap, over the base, so a player parked there
        // shooting up held off every enemy. With map cols 11-14 walled in the lower half (cell rows 0-12), the
        // corner spawns still get round to a player spawn.
        let centre = |cx, cy, t| hard_wall(cx, cy, t) || ((11..=14).contains(&cx) && cy <= 12);
        for (name, ter) in stages() {
            for spawn in [TOP[0], TOP[2]] {
                let d = steps(&ter, spawn, centre);
                assert!(PLAYERS.iter().any(|&(x, y)| d[y * CELLS + x].is_some()), "{name}: centre lane seals {spawn:?} off");
            }
        }
    }

    #[test]
    fn no_dead_end_pockets() {
        // Regression (owner): stage 6's side lanes ran from the top spawns down into a corner between steel and
        // water (17 cells, only way out back up), and enemies dithered there. Walking out of any dead end (a spot
        // with one way out) reaches a junction within MAX_DEPTH cells. Bricks count as open; a spawn may sit in one.
        const MAX_DEPTH: usize = 4;
        for (name, ter) in stages() {
            let fit = |x: usize, y: usize| fits(&ter, x, y, &hard_wall);
            let ways = |(x, y): (usize, usize)| neighbours(x, y).filter(|&(x, y)| fit(x, y)).collect::<Vec<_>>();
            for (x, y) in (0..CELLS).flat_map(|x| (0..CELLS).map(move |y| (x, y))) {
                if !fit(x, y) || ways((x, y)).len() > 1 || TOP.contains(&(x, y)) {
                    continue;
                }
                let (mut prev, mut cur, mut depth) = ((x, y), (x, y), 0);
                while let [next] = ways(cur).into_iter().filter(|&c| c != prev).collect::<Vec<_>>()[..] {
                    (prev, cur, depth) = (cur, next, depth + 1);
                    if ways(cur).len() > 2 {
                        break;
                    }
                }
                assert!(depth <= MAX_DEPTH, "{name}: dead end at cell {:?} is {depth} cells deep", (x, y));
            }
        }
    }

    #[test]
    fn stages_open_with_their_weakest_tanks() {
        // Regression: stage 1's shuffle sent a SCOUT first.
        for n in 1..=stage_count() * 2 {
            let roster: Vec<usize> = load(n).roster.iter().map(|k| k.index()).collect();
            let mut sorted = roster.clone();
            sorted.sort();
            assert_eq!(roster[..3], sorted[..3], "stage {n} opening wave");
        }
    }

    #[test]
    fn every_stage_parses() {
        for (i, src) in STAGES.iter().enumerate() {
            parse(src).unwrap_or_else(|e| panic!("levels/{:02}.txt: {e}", i + 1));
        }
    }

    #[test]
    fn half_rows_are_mirrored_and_spawns_cleared() {
        let row = "#@~%-........";
        let src = format!("enemies 20 0 0 0\n{}", format!("{row}\n").repeat(26));
        let (ter, counts) = parse(&src).unwrap();
        assert_eq!(counts, [20, 0, 0, 0]);
        assert_eq!(ter.get(0, 30), Tile::Brick);
        assert_eq!(ter.get(51, 30), Tile::Brick, "mirrored");
        assert_eq!(ter.get(2, 30), Tile::Steel);
        assert_eq!(ter.get(0, 51), Tile::Empty, "enemy spawn cleared");
        assert_eq!(ter.get(22, 0), Tile::Brick, "base ring");
    }

    #[test]
    fn bad_maps_are_rejected() {
        assert!(parse("enemies 10 0 0 0\n").is_err());
        assert!(parse("enemies 20 0 0 0\n.....\n").is_err());
    }

    #[test]
    fn roster_has_twenty_and_later_laps_get_harder() {
        let first = load(1);
        assert_eq!(first.roster.len(), ENEMIES_PER_STAGE);
        let basics = |s: &Stage| s.roster.iter().filter(|&&k| k == EnemyKind::Basic).count();
        let lapped = load(1 + stage_count());
        assert_eq!(lapped.roster.len(), ENEMIES_PER_STAGE);
        assert!(basics(&lapped) < basics(&first));
    }
}
