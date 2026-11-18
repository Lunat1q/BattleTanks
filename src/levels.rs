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
        let chars: Vec<char> = line.chars().collect();
        let row = match chars.len() {
            13 => chars.iter().chain(chars.iter().rev()).copied().collect(),
            26 => chars,
            n => return Err(format!("row {} has {n} cells, want 13 or 26", rows.len() + 1)),
        };
        rows.push(row);
    }
    if rows.len() != CELLS {
        return Err(format!("{} rows, want {CELLS}", rows.len()));
    }
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
            let (x, y) = (c * 2, (CELLS - 1 - r) * 2); // row 0 is the top
            for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                ter.set(x + dx, y + dy, tile);
            }
        }
    }
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

    /// Tank-sized flood fill over 8-unit cells (position = bottom-left cell of a 2x2 tank).
    /// Bricks count as open (they can be shot away); steel and water don't.
    fn reachable(ter: &Terrain, from: (usize, usize)) -> Vec<bool> {
        let open = |cx: usize, cy: usize| {
            (0..2).all(|dx| (0..2).all(|dy| !matches!(ter.get(cx * 2 + dx, cy * 2 + dy), Tile::Steel | Tile::Water)))
        };
        let fits = |x: usize, y: usize| x < CELLS - 1 && y < CELLS - 1 && (0..2).all(|dx| (0..2).all(|dy| open(x + dx, y + dy)));
        let mut seen = vec![false; CELLS * CELLS];
        let mut todo = vec![from];
        while let Some((x, y)) = todo.pop() {
            if !fits(x, y) || std::mem::replace(&mut seen[y * CELLS + x], true) {
                continue;
            }
            todo.extend([(x + 1, y), (x, y + 1)]);
            todo.extend(x.checked_sub(1).map(|x| (x, y)));
            todo.extend(y.checked_sub(1).map(|y| (x, y)));
        }
        seen
    }

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

    /// Shots a bullet fired straight down from the centre spawn needs before it reaches the base ring:
    /// each shot clears one 4-deep row of the 16-wide strip (sub-cells 24..28). Steel counts as a full cover.
    fn centre_lane_cover(ter: &Terrain) -> usize {
        let mut shots = 0;
        for y in 6..48 {
            let row: Vec<Tile> = (24..28).map(|x| ter.get(x, y)).collect();
            if row.contains(&Tile::Steel) {
                return usize::MAX;
            }
            shots += row.contains(&Tile::Brick) as usize;
        }
        shots
    }

    #[test]
    fn centre_spawn_has_no_free_shot_at_the_base() {
        // Regression: stage 1 had an open column from the centre spawn, so an enemy killed the base
        // in 3 shots (2 ring rows + base) before players could react.
        let open: Vec<String> = STAGES
            .iter()
            .enumerate()
            .map(|(i, src)| (i + 1, centre_lane_cover(&parse(src).unwrap().0)))
            .filter(|&(_, cover)| cover < 4)
            .map(|(n, cover)| format!("levels/{n:02}.txt ({cover} brick rows)"))
            .collect();
        assert!(open.is_empty(), "centre lane too open above the base ring: {open:?}");
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
