//! VERSUS maps (`levels/versus/NN.txt`, baked into the exe; ADR 0019). A file holds only the bottom 13 rows, in
//! the stage file's cells (`levels`); the top half is the bottom half turned 180 degrees, so cell `(c, r)` = cell
//! `(25 - c, 25 - r)` and both sides get the same map. Player 1's base, ring and spawn are forced at the bottom
//! (as in classic) and turned up for player 2. Pure, no ECS.

use crate::geom::{CELLS, SUBS};
use crate::levels::{paint, row};
use crate::terrain::{Terrain, Tile};

const MAPS: &[&str] =
    &[include_str!("../levels/versus/01.txt"), include_str!("../levels/versus/02.txt"), include_str!("../levels/versus/03.txt")];

pub fn count() -> usize {
    MAPS.len()
}

/// Map `n` (1-based, wraps).
pub fn load(n: usize) -> Terrain {
    let i = (n.max(1) - 1) % MAPS.len();
    let mut ter = parse(MAPS[i]).unwrap_or_else(|e| panic!("levels/versus/{:02}.txt: {e}", i + 1));
    ter.dirty.clear();
    ter
}

pub fn parse(src: &str) -> Result<Terrain, String> {
    let half: Vec<Vec<char>> = src
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with(';'))
        .enumerate()
        .map(|(n, l)| row(l, n))
        .collect::<Result<_, _>>()?;
    if half.len() != CELLS / 2 {
        return Err(format!("{} rows, want {} (the bottom half)", half.len(), CELLS / 2));
    }
    let top = half.iter().rev().map(|r| r.iter().rev().copied().collect());
    let mut ter = paint(&top.chain(half.iter().cloned()).collect::<Vec<_>>())?;
    // Player 1's spawn (x 64..80), base (96..112) and NPC corners clear, its ring brick; then the bottom half
    // turned up.
    for y in 0..4 {
        for x in (0..4).chain(16..20).chain(24..28).chain(SUBS - 4..SUBS) {
            ter.set(x, y, Tile::Empty);
        }
    }
    ter.set_base_ring(Tile::Brick);
    for y in 0..SUBS / 2 {
        for x in 0..SUBS {
            ter.set(SUBS - 1 - x, SUBS - 1 - y, ter.get(x, y));
        }
    }
    Ok(ter)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::levels::{fits, hard_wall, neighbours, steps};

    fn maps() -> impl Iterator<Item = (String, Terrain)> {
        MAPS.iter().enumerate().map(|(i, src)| (format!("levels/versus/{:02}.txt", i + 1), parse(src).unwrap()))
    }

    #[test]
    fn maps_are_point_symmetric_with_both_bases_ringed() {
        for (name, ter) in maps() {
            for (x, y) in (0..SUBS).flat_map(|x| (0..SUBS).map(move |y| (x, y))) {
                assert_eq!(ter.get(x, y), ter.get(SUBS - 1 - x, SUBS - 1 - y), "{name}: sub-cell {x},{y}");
            }
            assert!(Terrain::base_ring().all(|(x, y)| ter.get(x, y) == Tile::Brick), "{name}");
        }
        assert!(parse("..\n").is_err());
    }

    #[test]
    fn every_spawn_can_reach_the_other_base() {
        // Player 1 (cell 8, 0) and the NPC corners to just under player 2's ring (cell rows 23-25, cols 11-14);
        // player 2's side by symmetry.
        for (name, ter) in maps() {
            for spawn in [(8, 0), (0, 0), (24, 0)] {
                assert!(steps(&ter, spawn, hard_wall)[21 * CELLS + 12].is_some(), "{name}: {spawn:?} is walled off from the top base");
            }
        }
    }

    #[test]
    fn steel_covers_each_base_from_the_centre_lane() {
        // As in classic: every column over the ring has steel between the halfway line and the ring (the top by symmetry).
        for (name, ter) in maps() {
            assert!((22..30).all(|x| (6..SUBS / 2).any(|y| ter.get(x, y) == Tile::Steel)), "{name}: centre lane open to the base");
        }
    }

    #[test]
    fn no_free_shot_from_a_spawn_into_the_other_base() {
        // A spawn's straight lines (its columns up, its rows across) never cross the other base, map or not.
        let (spawn, other) = ((64.0..80.0, 0.0..16.0), (96.0..112.0, 192.0..208.0));
        let apart = |a: &std::ops::Range<f32>, b: &std::ops::Range<f32>| a.end <= b.start || b.end <= a.start;
        assert!(apart(&spawn.0, &other.0) && apart(&spawn.1, &other.1));
    }

    #[test]
    fn no_dead_end_pockets() {
        // Same rule as the classic stages: walking out of any dead end reaches a junction within 4 cells.
        for (name, ter) in maps() {
            let fit = |x: usize, y: usize| fits(&ter, x, y, &hard_wall);
            let ways = |(x, y): (usize, usize)| neighbours(x, y).filter(|&(x, y)| fit(x, y)).collect::<Vec<_>>();
            for (x, y) in (0..CELLS).flat_map(|x| (0..CELLS).map(move |y| (x, y))) {
                if !fit(x, y) || ways((x, y)).len() > 1 {
                    continue;
                }
                let (mut prev, mut cur, mut depth) = ((x, y), (x, y), 0);
                while let [next] = ways(cur).into_iter().filter(|&c| c != prev).collect::<Vec<_>>()[..] {
                    (prev, cur, depth) = (cur, next, depth + 1);
                    if ways(cur).len() > 2 {
                        break;
                    }
                }
                assert!(depth <= 4, "{name}: dead end at cell {:?} is {depth} cells deep", (x, y));
            }
        }
    }
}
