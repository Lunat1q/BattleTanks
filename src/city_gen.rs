//! Test-only City Run district generator: `cargo test write_city_districts -- --ignored` rewrites
//! `levels/city/NN.txt` (commit them; a rerun overwrites hand edits). Each district keeps its difficulty plan below
//! (height, turrets per bastion, mini-bosses, placed enemies, mix) and draws its streets with its theme's brushes
//! (`city_brushes.rs`, picked by the district name), so no stretch of street repeats. Bands are laid bottom-up; one that
//! fails `sound` (no tank-sized way up, a pocket only left by backing down, a turret no shell can reach; bricks count
//! as open, like the `city_map` tests) is redrawn. `committed_districts_are_sound` runs the same checks on the files.

use crate::city_brushes::{Band, Theme, W, band, rect, ri, theme};
use crate::rng::Rng;

struct Plan {
    name: &'static str,
    mix: [usize; 4],
    height: usize,
    /// Turrets per bastion, bottom first.
    turrets: &'static [usize],
    minis: usize,
    enemies: usize,
}

#[rustfmt::skip]
const PLANS: [Plan; 10] = [
    Plan { name: "Outskirts", mix: [8, 2, 0, 0], height: 390, turrets: &[2, 2, 2], minis: 2, enemies: 82 },
    Plan { name: "Market Row", mix: [7, 3, 1, 0], height: 413, turrets: &[2, 2, 2, 2], minis: 2, enemies: 74 },
    Plan { name: "Canal Ward", mix: [6, 3, 2, 0], height: 434, turrets: &[2, 2, 2, 2], minis: 3, enemies: 86 },
    Plan { name: "Old Town", mix: [5, 3, 2, 1], height: 465, turrets: &[4, 4, 4, 4, 4], minis: 3, enemies: 100 },
    Plan { name: "Ironworks", mix: [4, 3, 3, 1], height: 497, turrets: &[4, 4, 4, 4, 4], minis: 4, enemies: 108 },
    Plan { name: "Frost Park", mix: [3, 3, 3, 2], height: 523, turrets: &[4, 4, 4, 4, 4, 4], minis: 6, enemies: 106 },
    Plan { name: "Harbour", mix: [3, 3, 3, 3], height: 542, turrets: &[4, 4, 4, 6, 6, 6], minis: 6, enemies: 108 },
    Plan { name: "Rail Yards", mix: [2, 3, 4, 3], height: 568, turrets: &[4, 4, 6, 6, 6, 6, 6], minis: 5, enemies: 118 },
    Plan { name: "Citadel Gate", mix: [1, 3, 4, 4], height: 600, turrets: &[4, 4, 4, 6, 6, 6, 6], minis: 7, enemies: 124 },
    Plan { name: "The Citadel", mix: [1, 2, 4, 5], height: 616, turrets: &[4, 4, 4, 6, 6, 6, 6, 6], minis: 7, enemies: 110 },
];

const START: usize = 6;
/// Rows of the first screen: no stash of any kind there.
const FIRST_SCREEN: usize = 26;
/// Secret stashes per district.
const SECRETS: usize = 3;
/// Empty look-alike brick blocks per secret.
const DECOYS: usize = 3;
const ARENA: usize = 12;
/// Least boss-arena height; it grows by up to `TALLEST` so the district ends at exactly its planned height.
const BOSS: usize = 16;
/// Tallest street band any brush draws.
const TALLEST: usize = 12;
/// Rows in front of a turret kept free of steel (bricks and water are fine), so nobody has to hug the wall to hit it.
const APRON: usize = 6;
/// Rows a tank may have to back down to get out of a notch: the room the view keeps below the trailing tank.
const SLACK: usize = (crate::game::city::TRAIL_ROOM / crate::geom::CELL) as usize;

/// Laid rows (index 0 = bottom) and whether placed enemies may stand in each.
struct Map {
    rows: Band,
    street: Vec<bool>,
}

impl Map {
    /// Append the first band from `draw` that keeps the map `sound` (an open strip goes in after every 40 misses).
    fn lay(&mut self, rng: &mut Rng, street: bool, mut draw: impl FnMut(&mut Rng) -> Band) {
        for miss in 1.. {
            assert!(miss < 4000, "no band keeps the map sound at row {}", self.rows.len());
            let n = self.rows.len();
            self.rows.extend(draw(rng));
            if sound(&self.rows).is_ok() {
                break;
            }
            self.rows.truncate(n);
            if miss % 40 == 0 {
                self.rows.extend(band(2));
            }
        }
        self.street.resize(self.rows.len(), street);
    }
}

fn blocked(c: u8) -> bool {
    matches!(c, b'@' | b'~' | b'=' | b'T')
}

/// A 2x2 tank fits with its bottom-left on cell (x, y).
fn fits(rows: &Band, x: usize, y: usize) -> bool {
    x + 1 < W && y + 1 < rows.len() && (0..2).all(|d| (0..2).all(|e| !blocked(rows[y + e][x + d])))
}

/// Cells (bottom-left of a 2x2 tank) reachable from player 1's start, as in the `city_map` tests.
fn reach(rows: &Band) -> Vec<bool> {
    let h = rows.len();
    let fits = |x: usize, y: usize| fits(rows, x, y);
    let mut seen = vec![false; W * h];
    let mut todo = vec![(8, 0)];
    while let Some((x, y)) = todo.pop() {
        if !fits(x, y) || std::mem::replace(&mut seen[y * W + x], true) {
            continue;
        }
        todo.extend([(x + 1, y), (x, y + 1)]);
        todo.extend(x.checked_sub(1).map(|x| (x, y)));
        todo.extend(y.checked_sub(1).map(|y| (x, y)));
    }
    seen
}

/// Why the map so far can't be played to its top row, if it can't.
fn sound(rows: &Band) -> Result<(), String> {
    let r = reach(rows);
    let top = rows.len() - 2;
    if !(0..W).any(|x| r[top * W + x]) {
        return Err("no way up".into());
    }
    if let Some((x, y)) = trap(rows, &r) {
        return Err(format!("tank at column {x}, row {y} can only get out by backing down"));
    }
    if let Some((x, y)) = turrets(rows).into_iter().find(|&(x, y)| !hittable(rows, &r, x, y)) {
        return Err(format!("turret at column {x}, row {y} has no line of fire"));
    }
    Ok(())
}

/// A reachable cell from which the top row can't be reached without dropping more than `SLACK` rows below it. The
/// view never scrolls back down, and in co-op it stops rising at the trailing tank, so such a tank is stuck for good
/// and holds its partner back too (district 3 had a pocket beside a pond whose only way out ran under it).
/// Union-find, rows joined top-down: once row y is in, a cell of row y + SLACK linked to the top row climbs out
/// without ever going lower than y.
fn trap(rows: &Band, reach: &[bool]) -> Option<(usize, usize)> {
    fn root(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    let top = rows.len() - 2;
    let sky = W * rows.len();
    let mut p: Vec<usize> = (0..=sky).collect();
    let join = |p: &mut Vec<usize>, a: usize, b: usize| {
        let (a, b) = (root(p, a), root(p, b));
        p[a] = b;
    };
    let stuck = |p: &mut Vec<usize>, y: usize| (0..W).find(|&x| reach[y * W + x] && root(p, y * W + x) != root(p, sky)).map(|x| (x, y));
    for y in (0..=top).rev() {
        for x in (0..W).filter(|&x| fits(rows, x, y)) {
            let i = y * W + x;
            if y == top {
                join(&mut p, i, sky);
            } else if fits(rows, x, y + 1) {
                join(&mut p, i, i + W);
            }
            if x > 0 && fits(rows, x - 1, y) {
                join(&mut p, i, i - 1);
            }
        }
        // The bottom rows can only drop to row 0, so they are judged once everything is in.
        let judged = if y == 0 { 0..=SLACK.min(top) } else { y + SLACK..=y + SLACK };
        if let Some(c) = judged.filter(|&j| j <= top).find_map(|j| stuck(&mut p, j)) {
            return Some(c);
        }
    }
    None
}

/// Bottom-left cells of every turret (`TT/TT` squares; neighbours may touch).
fn turrets(rows: &Band) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for y in 0..rows.len() {
        let mut x = 0;
        while x < W {
            if rows[y][x] == b'T' && (y == 0 || rows[y - 1][x] != b'T') {
                out.push((x, y));
                x += 2;
            } else {
                x += 1;
            }
        }
    }
    out
}

/// No steel or wall in the `APRON` rows in front of the turret, across its columns and one either side (owner: no
/// steel that forces you to hug the turret), and a reachable tank square straight below it, close enough to share
/// the view, with a clear shot (bricks and water are fine).
fn hittable(rows: &Band, reach: &[bool], tx: usize, ty: usize) -> bool {
    let hard = |c: &u8| matches!(c, b'@' | b'=' | b'T');
    let apron = (ty.saturating_sub(APRON)..ty).all(|y| !rows[y][tx.saturating_sub(1)..(tx + 3).min(W)].iter().any(hard));
    let clear = |y: usize| !rows[y][tx..tx + 2].iter().any(hard);
    apron && (ty.saturating_sub(20)..ty.saturating_sub(1)).any(|y| reach[y * W + tx] && (y + 2..ty).all(clear))
}

/// A few 2x2 cover blocks at random spots outside `keep` (x0, y0, x1, y1).
fn cover(b: &mut Band, rng: &mut Rng, th: &Theme, n: usize, keep: (usize, usize, usize, usize)) {
    for _ in 0..n {
        let (x, y) = (ri(rng, 0, W - 2), ri(rng, 1, b.len() - 3));
        if x + 2 <= keep.0 || x >= keep.2 || y + 2 <= keep.1 || y >= keep.3 {
            rect(b, x, y, 2, 2, rng.pick(th.cover));
        }
    }
}

/// Open start with a little scenery at the edges, clear of both start squares.
fn start(rng: &mut Rng, th: &Theme) -> Band {
    let mut b = band(START);
    cover(&mut b, rng, th, 4, (4, 0, 22, START));
    b
}

/// Full-width fortress wall (rows 1-2) with a 4-wide gate and `n` turrets spread along it.
fn bastion(rng: &mut Rng, th: &Theme, n: usize) -> Band {
    let w = 1;
    let mut b = band(5);
    rect(&mut b, 0, w, W, 2, b'=');
    let g = ri(rng, 1, W - 5);
    rect(&mut b, g, w, 4, 2, b'G');
    let slots: Vec<usize> = (ri(rng, 0, 1)..W - 1).filter(|&x| x + 2 < g || x > g + 4).collect();
    let picked = (2..=5)
        .rev()
        .map(|gap| {
            let mut p: Vec<usize> = Vec::new();
            for &x in &slots {
                if p.last().is_none_or(|&l| x >= l + gap) {
                    p.push(x);
                }
            }
            p
        })
        .find(|p| p.len() >= n)
        .expect("room for the turrets");
    for i in 0..n {
        rect(&mut b, picked[i * picked.len() / n], w, 2, 2, b'T');
    }
    // Yard cover behind the wall, never in front of the gate.
    for _ in 0..2 {
        let x = ri(rng, 0, W - 2);
        if x + 2 <= g || x >= g + 4 {
            rect(&mut b, x, w + 2, 2, 2, rng.pick(th.cover));
        }
    }
    b
}

/// Open fight ground `h` rows tall holding a mini-boss (or the boss in its second row from the top), with cover.
fn arena(rng: &mut Rng, th: &Theme, h: usize, boss: bool) -> Band {
    let (size, mark) = if boss { (4, b'B') } else { (3, b'M') };
    let mut b = band(h);
    let x = ri(rng, 4, W - 4 - size);
    let y = if boss { h - size - 1 } else { ri(rng, 4, h - size - 3) };
    rect(&mut b, x, y, size, size, mark);
    let n = if boss { ri(rng, h / 2, h / 2 + 4) } else { ri(rng, 4, 7) };
    cover(&mut b, rng, th, n, (x - 2, y - 2, x + size + 2, y + size + 2));
    b
}

/// A street band with a pair of sealed brick stashes holding `kind` (`S` star, `L` extra tank).
fn stash(rng: &mut Rng, th: &Theme, kind: u8) -> Band {
    let mut b = (0..10).map(|_| street(rng, th)).find(|b| b.len() >= 6).unwrap_or_else(|| band(6));
    let x = ri(rng, 1, 7);
    for sx in [x, W - 4 - x] {
        rect(&mut b, sx - 1, 0, 6, 6, b'.');
        rect(&mut b, sx, 1, 4, 4, b'#');
        rect(&mut b, sx + 1, 2, 2, 2, kind);
    }
    b
}

fn street(rng: &mut Rng, th: &Theme) -> Band {
    let i = rng.below(th.brushes.len() as u32) as usize;
    (th.brushes[i])(rng)
}

enum Feature {
    Bastion(usize),
    Mini,
    Stash(u8),
}

fn district(p: &Plan, rng: &mut Rng) -> Band {
    let th = theme(p.name);
    let mut map = Map { rows: Band::new(), street: Vec::new() };
    map.lay(rng, false, |r| start(r, &th));
    let mut features: Vec<(f32, Feature)> = vec![(0.04, Feature::Stash(b'S')), (0.55, Feature::Stash(b'L'))];
    let b = p.turrets.len() as f32;
    features.extend(p.turrets.iter().enumerate().map(|(i, &t)| (0.1 + 0.82 * i as f32 / (b - 1.0), Feature::Bastion(t))));
    features.extend((0..p.minis).map(|j| (0.1 + 0.82 * (j as f32 + 0.5) / p.minis as f32, Feature::Mini)));
    features.sort_by(|a, c| a.0.total_cmp(&c.0));
    let body = (p.height - START - BOSS - TALLEST) as f32;
    for (f, feature) in features {
        // At least one street between two features.
        let floor = if matches!(feature, Feature::Stash(_)) { FIRST_SCREEN } else { 0 };
        loop {
            map.lay(rng, true, |r| street(r, &th));
            if map.rows.len() as f32 >= (START as f32 + f * body).max(floor as f32) {
                break;
            }
        }
        match feature {
            Feature::Bastion(n) => map.lay(rng, false, |r| bastion(r, &th, n)),
            Feature::Mini => map.lay(rng, false, |r| arena(r, &th, ARENA, false)),
            Feature::Stash(kind) => map.lay(rng, true, |r| stash(r, &th, kind)),
        }
    }
    // Exact height keeps the boss where the score server's district table expects it (`scores::CITY_FACTS`).
    while map.rows.len() + BOSS + TALLEST < p.height {
        map.lay(rng, true, |r| street(r, &th));
    }
    let h = p.height.saturating_sub(map.rows.len()).max(BOSS);
    map.lay(rng, false, |r| arena(r, &th, h, true));
    bury_secrets(&mut map, rng);
    place_enemies(&mut map, rng, p.enemies);
    // After the enemies, so their squares (and the server's district table) stay as they were.
    stamp_decoys(&mut map, rng);
    map.rows
}

/// Plain solid 4x4 brick blocks, `DECOYS` per secret slice, on open street ground: a lone block of brick is then
/// no tell that a secret sits inside (owner: "obvious that in that huge brick square there is something").
fn stamp_decoys(map: &mut Map, rng: &mut Rng) {
    let (lo, hi) = (FIRST_SCREEN, map.rows.len() - BOSS - 4);
    for i in 0..SECRETS * DECOYS {
        let rows = &map.rows;
        let open = |x: usize, y: usize| (y..y + 6).all(|r| map.street[r] && rows[r][x..x + 6].iter().all(|&v| v == b'.'));
        let pool: Vec<(usize, usize)> = (lo + (hi - lo) * i / (SECRETS * DECOYS)..lo + (hi - lo) * (i + 1) / (SECRETS * DECOYS))
            .flat_map(|y| (0..=W - 6).map(move |x| (x, y)))
            .filter(|&(x, y)| open(x, y))
            .collect();
        if let Some(&(x, y)) = pool.get(rng.below(pool.len().max(1) as u32) as usize) {
            rect(&mut map.rows, x + 1, y + 1, 4, 4, b'#');
        }
    }
}

/// `SECRETS` secret cores (`??/??`), one per slice of the climb above the first screen: inside a solid 4x4 brick block
/// the streets already have, or a stamped one on open ground where the slice has none.
fn bury_secrets(map: &mut Map, rng: &mut Rng) {
    let (lo, hi) = (FIRST_SCREEN, map.rows.len() - BOSS - 4);
    for i in 0..SECRETS {
        let rows = &map.rows;
        let area = |x: usize, y: usize, n: usize, c: u8| (y..y + n).all(|r| map.street[r] && rows[r][x..x + n].iter().all(|&v| v == c));
        let slice = || (lo + (hi - lo) * i / SECRETS..lo + (hi - lo) * (i + 1) / SECRETS).flat_map(|y| (0..W - 3).map(move |x| (x, y)));
        let natural: Vec<(usize, usize)> = slice().filter(|&(x, y)| area(x, y, 4, b'#')).collect();
        let open: Vec<(usize, usize)> = slice().filter(|&(x, y)| x > 0 && x + 5 <= W && area(x - 1, y - 1, 6, b'.')).collect();
        let pool = if natural.is_empty() { open } else { natural };
        if pool.is_empty() {
            continue;
        }
        let (x, y) = pool[rng.below(pool.len() as u32) as usize];
        rect(&mut map.rows, x, y, 4, 4, b'#');
        rect(&mut map.rows, x + 1, y + 1, 2, 2, b'?');
    }
}

/// `n` enemy squares on reachable open street ground, one per even slice of the climb (a slice with no room passes
/// its enemy on to the next).
fn place_enemies(map: &mut Map, rng: &mut Rng, n: usize) {
    let reach = reach(&map.rows);
    let (lo, hi) = (START + 8, map.rows.len() - BOSS);
    let mut owed = 0;
    // One pass per slice, then a last pass over the whole climb for any still owed.
    for i in (0..n).chain([n]) {
        owed += usize::from(i < n);
        while owed > 0 {
            let rows = &map.rows;
            let fits = |x: usize, y: usize| {
                let near = (y.saturating_sub(1)..(y + 3).min(rows.len()))
                    .all(|yy| (x.saturating_sub(1)..(x + 3).min(W)).all(|xx| rows[yy][xx] != b'E'));
                reach[y * W + x] && map.street[y] && map.street[y + 1] && near && (0..2).all(|e| rows[y + e][x..x + 2] == *b"..")
            };
            let slice = if i < n { lo + (hi - lo) * i / n..lo + (hi - lo) * (i + 1) / n } else { lo..hi };
            let spots: Vec<(usize, usize)> = slice.flat_map(|y| (0..W - 1).map(move |x| (x, y))).filter(|&(x, y)| fits(x, y)).collect();
            if spots.is_empty() {
                break;
            }
            let (x, y) = spots[rng.below(spots.len() as u32) as usize];
            rect(&mut map.rows, x, y, 2, 2, b'E');
            owed -= 1;
        }
    }
}

fn render(n: usize, p: &Plan, rows: &Band) -> String {
    let [a, b, c, d] = p.mix;
    let mut s = format!(
        "; City Run district {n} - {}. Bottom row = start; see city_map.rs for the format.\n\
         ; Drawn by `cargo test write_city_districts -- --ignored` (city_gen.rs); a rerun overwrites hand edits.\n\
         name {}\nmix {a} {b} {c} {d}\n",
        p.name, p.name
    );
    for r in rows.iter().rev() {
        s.push_str(std::str::from_utf8(r).unwrap());
        s.push('\n');
    }
    s
}

/// Map rows of a district file, bottom first.
fn rows_of(text: &str) -> Band {
    let mut rows: Band = text.lines().filter_map(|l| l.as_bytes().try_into().ok()).collect();
    rows.reverse();
    rows
}

#[test]
fn committed_districts_are_sound() {
    for n in 1..=PLANS.len() {
        let text = std::fs::read_to_string(format!("levels/city/{n:02}.txt")).unwrap();
        sound(&rows_of(&text)).unwrap_or_else(|e| panic!("district {n}: {e}"));
    }
}

#[test]
fn the_validator_catches_both_owner_reports() {
    // District 4: steel straight under a turret. District 3: a pocket beside a pond, its only exit under the pond.
    let mut walled = band(12);
    rect(&mut walled, 0, 8, W, 2, b'=');
    rect(&mut walled, 12, 8, 4, 2, b'G');
    rect(&mut walled, 4, 8, 2, 2, b'T');
    assert!(sound(&walled).is_ok());
    rect(&mut walled, 4, 0, 2, 8, b'@');
    assert!(sound(&walled).unwrap_err().contains("turret"));
    let mut pond = band(12);
    rect(&mut pond, 0, 8, W, 2, b'~');
    rect(&mut pond, 4, 8, 3, 2, b'.');
    rect(&mut pond, 14, 2, 4, 4, b'~');
    assert!(sound(&pond).is_ok());
    rect(&mut pond, 14, 2, 4, 6, b'~');
    assert!(sound(&pond).unwrap_err().contains("backing down"));
}

#[test]
#[ignore]
fn write_city_districts() {
    for (i, p) in PLANS.iter().enumerate() {
        let mut rng = Rng::new(0xC17E + i as u64 * 7919);
        let text = render(i + 1, p, &district(p, &mut rng));
        crate::city_map::parse(&text, 0).unwrap_or_else(|e| panic!("district {}: {e}", i + 1));
        sound(&rows_of(&text)).unwrap_or_else(|e| panic!("district {}: {e}", i + 1));
        std::fs::write(format!("levels/city/{:02}.txt", i + 1), text).unwrap();
    }
}
