//! Street brushes for the City Run district generator (`city_gen.rs`), one set per district theme. Test-only.
//! A brush paints a band of full-width rows (index 0 = bottom row) and picks its own height; the generator redraws a
//! band that leaves no tank-sized way up, so brushes may be bold.

use crate::rng::Rng;

mod inner;
mod outer;
use inner::*;
use outer::*;

pub const W: usize = 26;
pub type Band = Vec<[u8; W]>;
pub type Brush = fn(&mut Rng) -> Band;

pub struct Theme {
    pub brushes: Vec<Brush>,
    /// Cover blocks in mini-boss and boss arenas and bastion yards.
    pub cover: &'static [u8],
}

/// The district's look, from its name.
pub fn theme(name: &str) -> Theme {
    let (brushes, cover): (Vec<Brush>, &[u8]) = match name {
        "Outskirts" => (vec![fields as Brush, hedgerow, farm, pond, lane], b"%#"),
        "Market Row" => (vec![stalls as Brush, arcade, plaza, crates], b"#"),
        "Canal Ward" => (vec![canal as Brush, bend, terrace, lock], b"#~"),
        "Old Town" => (vec![quarter as Brush, square, church, town_wall], b"#%"),
        "Ironworks" => (vec![hall as Brush, pipes, yard, cooling, conveyor], b"@#"),
        "Frost Park" => (vec![ice_field as Brush, frozen_pond, grove, hedges], b"-%"),
        "Harbour" => (vec![basin as Brush, containers, cranes, quay], b"@#"),
        "Rail Yards" => (vec![tracks as Brush, platform, depot, signals], b"#-"),
        "Citadel Gate" => (vec![trench as Brush, bunkers, teeth, ramparts], b"@#"),
        "The Citadel" => (vec![courtyard as Brush, maze, moat, pillars], b"@#"),
        _ => panic!("no theme for district {name:?}"),
    };
    Theme { brushes, cover }
}

/// Inclusive random range.
pub fn ri(rng: &mut Rng, lo: usize, hi: usize) -> usize {
    lo + rng.below((hi - lo + 1) as u32) as usize
}

pub fn band(h: usize) -> Band {
    vec![[b'.'; W]; h]
}

/// Fill a rectangle, clipped to the band.
pub fn rect(b: &mut Band, x: usize, y: usize, w: usize, h: usize, c: u8) {
    for row in b.iter_mut().skip(y).take(h) {
        row.iter_mut().skip(x).take(w).for_each(|cell| *cell = c);
    }
}

fn set(b: &mut Band, x: usize, y: usize, c: u8) {
    rect(b, x, y, 1, 1, c);
}

/// A rectangle with randomly nibbled corners, so patches look grown rather than stamped.
fn patch(b: &mut Band, rng: &mut Rng, x: usize, y: usize, w: usize, h: usize, c: u8) {
    rect(b, x, y, w, h, c);
    if w > 2 && h > 1 {
        for (cx, cy) in [(x, y), (x + w - 1, y), (x, y + h - 1), (x + w - 1, y + h - 1)] {
            if rng.chance(0.5) {
                set(b, cx, cy, b'.');
            }
        }
    }
}

/// A full-width wall `h` rows tall with `gaps` random openings 2-3 cells wide.
fn wall(b: &mut Band, rng: &mut Rng, y: usize, h: usize, c: u8, gaps: usize) {
    rect(b, 0, y, W, h, c);
    for _ in 0..gaps {
        let w = ri(rng, 2, 3);
        let x = ri(rng, 0, W - w);
        rect(b, x, y, w, h, b'.');
    }
}

/// `n` blocks of `w`x`h` at random spots.
fn scatter(b: &mut Band, rng: &mut Rng, n: usize, w: usize, h: usize, c: u8) {
    let bh = b.len();
    for _ in 0..n {
        let (x, y) = (ri(rng, 0, W - w), ri(rng, 0, bh - h));
        rect(b, x, y, w, h, c);
    }
}

/// Random grown patches of `c`, `w`/`h` ranges inclusive.
fn patches(b: &mut Band, rng: &mut Rng, n: usize, w: (usize, usize), h: (usize, usize), c: u8) {
    for _ in 0..n {
        let (pw, ph) = (ri(rng, w.0, w.1), ri(rng, h.0, h.1).min(b.len()));
        let (x, y) = (ri(rng, 0, W - pw), ri(rng, 0, b.len() - ph));
        patch(b, rng, x, y, pw, ph, c);
    }
}

/// Full-width water `h` rows tall at `y`, crossed by `n` bridges 3 wide.
fn river(b: &mut Band, rng: &mut Rng, y: usize, h: usize, n: usize) -> Vec<usize> {
    rect(b, 0, y, W, h, b'~');
    let xs: Vec<usize> = (0..n).map(|_| ri(rng, 1, W - 4)).collect();
    xs.iter().for_each(|&x| rect(b, x, y, 3, h, b'.'));
    xs
}
