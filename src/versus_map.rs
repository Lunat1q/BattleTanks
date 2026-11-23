//! VERSUS maps (ADR 0019). A map is a bottom half of 13 rows, in the stage file's cells (`levels`); the top half
//! is it turned 180 degrees, so cell `(c, r)` = cell `(25 - c, 25 - r)` and both sides get the same map. Player 1's
//! base, ring and spawn are forced at the bottom (as in classic) and turned up for player 2. The maps: the
//! hand-drawn `levels/versus/NN.txt` first, then the bottom half of every classic stage that passes `sound`
//! (owner: stage maps mirrored, so there are plenty). Pure, no ECS.

use crate::geom::{CELLS, SUBS};
use crate::levels::{fits, hard_wall, neighbours, paint, row, steps};
use crate::terrain::{Terrain, Tile};
use std::sync::OnceLock;

const MAPS: &[&str] =
    &[include_str!("../levels/versus/01.txt"), include_str!("../levels/versus/02.txt"), include_str!("../levels/versus/03.txt")];

/// Every map, built once: the same list on every machine (LAN).
fn all() -> &'static [Terrain] {
    static ALL: OnceLock<Vec<Terrain>> = OnceLock::new();
    ALL.get_or_init(|| {
        let drawn = MAPS.iter().enumerate().map(|(i, src)| parse(src).unwrap_or_else(|e| panic!("levels/versus/{:02}.txt: {e}", i + 1)));
        let stages = (1..=crate::levels::stage_count()).filter_map(|n| parse(&stage_half(crate::levels::source(n))).ok());
        drawn.chain(stages.filter(|t| sound(t).is_ok())).collect()
    })
}

pub fn count() -> usize {
    all().len()
}

/// Map `n` (1-based, wraps).
pub fn load(n: usize) -> Terrain {
    let mut ter = all()[(n.max(1) - 1) % count()].clone();
    ter.dirty.clear();
    ter
}

/// A classic stage file's bottom 13 map rows.
fn stage_half(src: &str) -> String {
    let rows: Vec<&str> =
        src.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with(';') && !l.starts_with("enemies")).collect();
    rows[rows.len().saturating_sub(CELLS / 2)..].join(
        "
",
    )
}

/// The VERSUS map rules (player 2's side follows by symmetry):
/// - player 1 (cell 8, 0) and the NPC corners reach just under player 2's ring (cell rows 23-25, cols 11-14),
///   bricks open;
/// - every column over the ring has steel between the halfway line and the ring, as in classic, so nobody shoots a
///   base straight down the centre lane;
/// - walking out of any dead end reaches a junction within 4 cells (no pockets), as in classic.
pub fn sound(ter: &Terrain) -> Result<(), String> {
    for spawn in [(8, 0), (0, 0), (24, 0)] {
        if steps(ter, spawn, hard_wall)[21 * CELLS + 12].is_none() {
            return Err(format!("{spawn:?} is walled off from the top base"));
        }
    }
    if !(22..30).all(|x| (6..SUBS / 2).any(|y| ter.get(x, y) == Tile::Steel)) {
        return Err("centre lane open to the base".into());
    }
    let fit = |x: usize, y: usize| fits(ter, x, y, &hard_wall);
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
        if depth > 4 {
            return Err(format!("dead end at cell {:?} is {depth} cells deep", (x, y)));
        }
    }
    Ok(())
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

    #[test]
    fn every_map_is_point_symmetric_ringed_and_sound() {
        for n in 1..=count() {
            let ter = load(n);
            for (x, y) in (0..SUBS).flat_map(|x| (0..SUBS).map(move |y| (x, y))) {
                assert_eq!(ter.get(x, y), ter.get(SUBS - 1 - x, SUBS - 1 - y), "map {n}: sub-cell {x},{y}");
            }
            assert!(Terrain::base_ring().all(|(x, y)| ter.get(x, y) == Tile::Brick), "map {n}");
            assert_eq!(sound(&ter), Ok(()), "map {n}");
        }
        for (i, src) in MAPS.iter().enumerate() {
            assert_eq!(sound(&parse(src).unwrap()), Ok(()), "levels/versus/{:02}.txt", i + 1);
        }
        assert!(
            parse(
                "..
"
            )
            .is_err()
        );
    }

    #[test]
    fn classic_stages_add_maps() {
        let report: Vec<String> = (1..=crate::levels::stage_count())
            .map(|n| format!("stage {n}: {:?}", parse(&stage_half(crate::levels::source(n))).and_then(|t| sound(&t))))
            .collect();
        assert!(
            count() >= MAPS.len() + 8,
            "{}",
            report.join(
                "
"
            )
        );
        assert_eq!(stage_half(crate::levels::source(1)).lines().count(), CELLS / 2);
    }

    #[test]
    fn no_free_shot_from_a_spawn_into_the_other_base() {
        // A spawn's straight lines (its columns up, its rows across) never cross the other base, map or not.
        let (spawn, other) = ((64.0..80.0, 0.0..16.0), (96.0..112.0, 192.0..208.0));
        let apart = |a: &std::ops::Range<f32>, b: &std::ops::Range<f32>| a.end <= b.start || b.end <= a.start;
        assert!(apart(&spawn.0, &other.0) && apart(&spawn.1, &other.1));
    }
}
