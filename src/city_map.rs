//! City Run district files (`levels/city/NN.txt`, baked into the exe) -> tall terrain + placed enemies,
//! turrets, mini-bosses, the boss and bastion gates. Pure, no ECS.
//!
//! Format (rows top first, the bottom row is where the players start):
//! ```text
//! ; comment
//! name Harbour Gate
//! mix 8 2 0 0           <- weights for placed `E` enemies: basic fast power armor
//! .............          <- 13 chars = left half, mirrored; 26 chars = full row; at least 26 rows
//! ```
//! Cells as in classic maps (`.#@~%-`), plus `=` fortress wall and `G` gate (both unbreakable).
//! Markers are drawn as full squares so mirroring keeps them whole: `EE/EE` enemy, `TT/TT` turret,
//! `MMM` x3 mini-boss, `BBBB` x4 boss (exactly one). A gate opens once every turret nearest to it
//! (vertically) is destroyed; a dead turret's spot becomes wall.

use crate::geom::{CELL, CELLS, SUBS};
use crate::rng::Rng;
use crate::rules::{CITY_BONUS_EVERY, CITY_DISTRICTS, EnemyKind, Rank};
use crate::terrain::{Terrain, Tile};
use bevy::math::Vec2;

const DISTRICTS: [&str; CITY_DISTRICTS] = [
    include_str!("../levels/city/01.txt"),
    include_str!("../levels/city/02.txt"),
    include_str!("../levels/city/03.txt"),
    include_str!("../levels/city/04.txt"),
    include_str!("../levels/city/05.txt"),
    include_str!("../levels/city/06.txt"),
    include_str!("../levels/city/07.txt"),
    include_str!("../levels/city/08.txt"),
    include_str!("../levels/city/09.txt"),
    include_str!("../levels/city/10.txt"),
];

/// Player start columns (x centres), bottom row of every district.
pub const STARTS: [f32; 2] = [72.0, 136.0];

#[derive(Clone, Debug, PartialEq)]
pub struct Spawn {
    pub rank: Rank,
    pub kind: EnemyKind,
    /// Centre in field units.
    pub pos: Vec2,
    /// Turrets: the bastion whose gate they guard.
    pub bastion: Option<usize>,
    pub bonus: bool,
}

#[derive(Clone)]
pub struct District {
    /// Weights per `EnemyKind` for placed enemies and reinforcements.
    pub mix: [usize; 4],
    pub terrain: Terrain,
    /// Sorted bottom to top, the order they come into view.
    pub spawns: Vec<Spawn>,
    /// Gate sub-cells per bastion.
    pub gates: Vec<Vec<(usize, usize)>>,
}

fn marker(ch: char) -> Option<(Rank, usize)> {
    match ch {
        'E' => Some((Rank::Normal, 2)),
        'T' => Some((Rank::Turret, 2)),
        'M' => Some((Rank::Mini, 3)),
        'B' => Some((Rank::Boss, 4)),
        _ => None,
    }
}

/// Sub-cells of a cell.
fn subs(c: usize, row_from_bottom: usize) -> [(usize, usize); 4] {
    let (x, y) = (c * 2, row_from_bottom * 2);
    [(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)]
}

pub fn parse(src: &str, seed: u64) -> Result<District, String> {
    let mut mix = None;
    let mut rows: Vec<Vec<char>> = Vec::new();
    for line in src.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with(';')) {
        if line.starts_with("name ") {
            continue; // read by `name()`
        }
        if let Some(rest) = line.strip_prefix("mix") {
            let n: Vec<usize> =
                rest.split_whitespace().map(|s| s.parse().map_err(|_| format!("bad mix {s:?}"))).collect::<Result<_, _>>()?;
            let n: [usize; 4] = n.try_into().map_err(|_| "mix needs 4 weights".to_string())?;
            if n.iter().sum::<usize>() == 0 {
                return Err("mix weights are all zero".into());
            }
            mix = Some(n);
            continue;
        }
        let chars: Vec<char> = line.chars().collect();
        rows.push(match chars.len() {
            13 => chars.iter().chain(chars.iter().rev()).copied().collect(),
            26 => chars,
            n => return Err(format!("row {} has {n} cells, want 13 or 26", rows.len() + 1)),
        });
    }
    let mix = mix.ok_or("missing `mix` line")?;
    let h = rows.len();
    if h < CELLS {
        return Err(format!("{h} rows, want at least {CELLS}"));
    }
    let mut ter = Terrain::new(h * 2);
    let mut claimed = vec![false; h * CELLS];
    let mut spawns = Vec::new();
    let mut gate_cells = Vec::new();
    for r in 0..h {
        for c in 0..CELLS {
            let ch = rows[r][c];
            let tile = match ch {
                '.' | 'E' | 'T' | 'M' | 'B' => Tile::Empty,
                '#' => Tile::Brick,
                '@' => Tile::Steel,
                '~' => Tile::Water,
                '%' => Tile::Trees,
                '-' => Tile::Ice,
                '=' => Tile::Wall,
                'G' => Tile::Gate,
                other => return Err(format!("unknown cell {other:?} at row {}", r + 1)),
            };
            for (x, y) in subs(c, h - 1 - r) {
                ter.set(x, y, tile);
            }
            if ch == 'G' {
                gate_cells.push((c, r));
            }
            let Some((rank, n)) = marker(ch) else { continue };
            if claimed[r * CELLS + c] {
                continue;
            }
            for dr in 0..n {
                for dc in 0..n {
                    let ok = r + dr < h && c + dc < CELLS && rows[r + dr][c + dc] == ch && !claimed[(r + dr) * CELLS + c + dc];
                    if !ok {
                        return Err(format!("`{ch}` at row {} col {} must be a {n}x{n} square", r + 1, c + 1));
                    }
                    claimed[(r + dr) * CELLS + c + dc] = true;
                }
            }
            let half = n as f32 * CELL / 2.0;
            let pos = Vec2::new(c as f32 * CELL + half, (h - r - n) as f32 * CELL + half);
            spawns.push(Spawn { rank, kind: EnemyKind::Basic, pos, bastion: None, bonus: false });
        }
    }
    // Player starts are always open.
    for x in STARTS {
        let c0 = (x / CELL) as usize - 1;
        for c in c0..c0 + 2 {
            for r in 0..2 {
                for (sx, sy) in subs(c, r) {
                    ter.set(sx, sy, Tile::Empty);
                }
            }
        }
    }
    if spawns.iter().filter(|s| s.rank == Rank::Boss).count() != 1 {
        return Err("need exactly one boss (BBBB x4)".into());
    }
    let gates = group_gates(&gate_cells, h);
    // Turret -> nearest gate by vertical distance.
    let gate_y: Vec<f32> = gates.iter().map(|g| g.iter().map(|&(_, y)| y as f32 * 4.0 + 2.0).sum::<f32>() / g.len() as f32).collect();
    let mut guarded = vec![false; gates.len()];
    let mut rng = Rng::new(seed);
    spawns.sort_by(|a, b| a.pos.y.total_cmp(&b.pos.y));
    let mut normal = 0;
    for s in &mut spawns {
        match s.rank {
            Rank::Normal => {
                s.kind = weighted(&mix, &mut rng);
                normal += 1;
                s.bonus = normal % CITY_BONUS_EVERY == 0;
            }
            Rank::Turret => {
                s.kind = EnemyKind::Armor;
                s.bastion = (0..gates.len()).min_by(|&a, &b| (gate_y[a] - s.pos.y).abs().total_cmp(&(gate_y[b] - s.pos.y).abs()));
                if let Some(b) = s.bastion {
                    guarded[b] = true;
                }
            }
            Rank::Mini => s.kind = EnemyKind::Armor,
            Rank::Boss => s.kind = EnemyKind::Power,
        }
    }
    if let Some(i) = guarded.iter().position(|g| !g) {
        return Err(format!("gate {} has no turret, it would never open", i + 1));
    }
    ter.dirty.clear();
    Ok(District { mix, terrain: ter, spawns, gates })
}

pub fn weighted(mix: &[usize; 4], rng: &mut Rng) -> EnemyKind {
    let mut roll = rng.below(mix.iter().sum::<usize>() as u32) as usize;
    for (i, &w) in mix.iter().enumerate() {
        if roll < w {
            return EnemyKind::ALL[i];
        }
        roll -= w;
    }
    EnemyKind::Basic
}

/// 4-connected groups of gate cells -> their sub-cells (y from the bottom).
fn group_gates(cells: &[(usize, usize)], h: usize) -> Vec<Vec<(usize, usize)>> {
    let mut left: Vec<(usize, usize)> = cells.to_vec();
    let mut groups = Vec::new();
    while let Some(first) = left.pop() {
        let mut group = vec![first];
        let mut i = 0;
        while i < group.len() {
            let (c, r) = group[i];
            let near = |&(c2, r2): &(usize, usize)| c.abs_diff(c2) + r.abs_diff(r2) == 1;
            let (found, rest): (Vec<_>, Vec<_>) = left.iter().partition(|p| near(p));
            left = rest;
            group.extend(found);
            i += 1;
        }
        groups.push(group.iter().flat_map(|&(c, r)| subs(c, h - 1 - r)).collect());
    }
    groups
}

/// District `n` (1-based, clamped).
pub fn load(n: usize) -> District {
    let i = n.clamp(1, CITY_DISTRICTS) - 1;
    parse(DISTRICTS[i], i as u64 * 104_729 + 17).unwrap_or_else(|e| panic!("levels/city/{:02}.txt: {e}", i + 1))
}

/// Display name of district `n` without building it.
pub fn name(n: usize) -> &'static str {
    let src = DISTRICTS[n.clamp(1, CITY_DISTRICTS) - 1];
    src.lines().find_map(|l| l.trim().strip_prefix("name ")).map_or("", str::trim)
}

/// A spawn's cells in sub-cell units (for filling a dead turret's spot).
pub fn footprint(pos: Vec2, size: f32) -> impl Iterator<Item = (usize, usize)> {
    let (x0, y0) = (((pos.x - size / 2.0) / 4.0) as usize, ((pos.y - size / 2.0) / 4.0) as usize);
    let n = (size / 4.0) as usize;
    (y0..y0 + n).flat_map(move |y| (x0..(x0 + n).min(SUBS)).map(move |x| (x, y)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tank-sized flood fill over 8-unit cells from cell `from`; bricks open, steel/water/wall closed,
    /// gates open unless listed in `closed`.
    fn reachable(d: &District, from: (usize, usize), closed: &[usize]) -> Vec<bool> {
        let mut ter = d.terrain.clone();
        for s in d.spawns.iter().filter(|s| s.rank == Rank::Turret) {
            footprint(s.pos, 16.0).for_each(|(x, y)| ter.set(x, y, Tile::Wall));
        }
        for (i, g) in d.gates.iter().enumerate() {
            let t = if closed.contains(&i) { Tile::Wall } else { Tile::Empty };
            g.iter().for_each(|&(x, y)| ter.set(x, y, t));
        }
        let h = ter.rows() / 2;
        let open = |cx: usize, cy: usize| {
            (0..2).all(|dx| (0..2).all(|dy| !matches!(ter.get(cx * 2 + dx, cy * 2 + dy), Tile::Steel | Tile::Water | Tile::Wall)))
        };
        let fits = |x: usize, y: usize| x < CELLS - 1 && y < h - 1 && (0..2).all(|dx| (0..2).all(|dy| open(x + dx, y + dy)));
        let mut seen = vec![false; CELLS * h];
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

    /// Bottom-left cell of a spawn's square.
    fn cell_of(s: &Spawn) -> (usize, usize) {
        let half = s.rank.size() / 2.0;
        (((s.pos.x - half) / CELL) as usize, ((s.pos.y - half) / CELL) as usize)
    }

    #[test]
    fn every_district_parses_and_is_winnable() {
        for n in 1..=CITY_DISTRICTS {
            let d = load(n);
            assert!(!name(n).is_empty(), "district {n} has no name");
            let reach = reachable(&d, (8, 0), &[]);
            assert!(reach[16], "district {n}: player 2 start walled off");
            for s in d.spawns.iter().filter(|s| matches!(s.rank, Rank::Mini | Rank::Boss)) {
                let (x, y) = cell_of(s);
                assert!(reach[y * CELLS + x], "district {n}: {:?} at {} unreachable", s.rank, s.pos);
            }
        }
    }

    #[test]
    fn every_bastion_actually_bars_the_way() {
        for n in 1..=CITY_DISTRICTS {
            let d = load(n);
            assert!(!d.gates.is_empty(), "district {n} has no bastion");
            let boss = d.spawns.iter().find(|s| s.rank == Rank::Boss).unwrap();
            let (x, y) = cell_of(boss);
            for g in 0..d.gates.len() {
                assert!(!reachable(&d, (8, 0), &[g])[y * CELLS + x], "district {n}: bastion {} can be walked around", g + 1);
            }
        }
    }

    #[test]
    fn markers_become_spawns() {
        let mut src = String::from("name Test\nmix 0 1 0 0\n");
        src += &"...........BB\n".repeat(4);
        src += &"=========TTGG\n".repeat(2);
        src += &".............\n".repeat(16);
        src += "EE...........\nEE...........\n.............\n.............\n";
        let d = parse(&src, 1).unwrap();
        let ranks: Vec<Rank> = d.spawns.iter().map(|s| s.rank).collect();
        assert_eq!(ranks, [Rank::Normal, Rank::Normal, Rank::Turret, Rank::Turret, Rank::Boss]);
        assert_eq!(d.spawns[0].pos, Vec2::new(8.0, 24.0));
        assert_eq!(d.spawns[0].kind, EnemyKind::Fast, "mix picks the only weighted kind");
        assert_eq!(d.spawns[4].pos, Vec2::new(104.0, 26.0 * 8.0 - 16.0));
        assert_eq!(d.gates.len(), 1, "the mirrored gate halves touch: one gate");
        assert!(d.spawns[2..4].iter().all(|s| s.bastion == Some(0)));
        assert_eq!(d.terrain.get(0, 42), Tile::Wall);
        assert_eq!(d.terrain.get(22, 42), Tile::Gate);
        assert_eq!(d.terrain.get(18, 42), Tile::Empty, "turret spot stays open while it lives");
    }

    #[test]
    fn broken_markers_and_lonely_gates_are_errors() {
        let base = format!("name X\nmix 1 0 0 0\n{}", "...........BB\n".repeat(4));
        let rows = |n: usize| ".............\n".repeat(n);
        assert!(parse(&format!("{base}{}", rows(22)), 0).is_ok());
        assert!(parse(&format!("{base}E............\n{}", rows(21)), 0).err().unwrap().contains("square"));
        assert!(parse(&format!("{base}=====GG======\n{}", rows(21)), 0).err().unwrap().contains("no turret"));
        assert!(parse(&format!("name X\nmix 1 0 0 0\n{}", rows(26)), 0).err().unwrap().contains("boss"));
    }

    #[test]
    fn footprint_covers_the_square() {
        let cells: Vec<_> = footprint(Vec2::new(16.0, 24.0), 16.0).collect();
        assert_eq!(cells.len(), 16);
        assert!(cells.contains(&(2, 4)) && cells.contains(&(5, 7)));
    }
}
