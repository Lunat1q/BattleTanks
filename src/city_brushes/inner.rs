//! Brushes for the inner districts 6-10 (see `city_brushes`).

use super::{Band, W, band, patches, rect, ri, river, scatter, set, wall};
use crate::rng::Rng;

// --- Frost Park: ice fields, frozen ponds, groves, snowy hedges.

pub fn ice_field(rng: &mut Rng) -> Band {
    let h = ri(rng, 6, 9);
    let mut b = band(h);
    rect(&mut b, 0, 0, W, h, b'-');
    let (t, o) = (ri(rng, 2, 4), ri(rng, 1, 3));
    patches(&mut b, rng, t, (2, 4), (2, 3), b'%');
    patches(&mut b, rng, o, (3, 5), (2, 2), b'.');
    b
}

pub fn frozen_pond(rng: &mut Rng) -> Band {
    let mut b = band(7);
    let (w, h) = (ri(rng, 4, 8), ri(rng, 2, 3));
    let x = ri(rng, 1, W - w - 1);
    rect(&mut b, x - 1, 1, w + 2, h + 2, b'-');
    rect(&mut b, x, 2, w, h, b'~');
    let n = ri(rng, 2, 4);
    scatter(&mut b, rng, n, 1, 2, b'%');
    b
}

pub fn grove(rng: &mut Rng) -> Band {
    let mut b = band(6);
    let n = ri(rng, 3, 5);
    patches(&mut b, rng, n, (2, 5), (2, 4), b'%');
    b
}

pub fn hedges(rng: &mut Rng) -> Band {
    let mut b = band(4);
    rect(&mut b, 0, 0, W, 1, b'-');
    for _ in 0..ri(rng, 3, 4) {
        let w = ri(rng, 3, 6);
        let (x, y) = (ri(rng, 0, W - w), ri(rng, 1, 2));
        rect(&mut b, x, y, w, 1, b'#');
    }
    b
}

// --- Harbour: basins with piers, container stacks, cranes, a quay.

pub fn basin(rng: &mut Rng) -> Band {
    let mut b = band(8);
    let w = ri(rng, 8, 12);
    let left = rng.chance(0.5);
    let x = if left { 0 } else { W - w };
    rect(&mut b, x, 1, w, 6, b'~');
    let mut px = x + ri(rng, 1, 2);
    while px + 2 <= x + w {
        let len = ri(rng, 2, 5);
        rect(&mut b, px, 1, 2, len, b'.');
        set(&mut b, px, len, b'@');
        px += ri(rng, 4, 5);
    }
    for _ in 0..ri(rng, 2, 4) {
        let cx = if left { ri(rng, w + 1, W - 4) } else { ri(rng, 0, W - w - 5) };
        let cy = ri(rng, 1, 5);
        rect(&mut b, cx, cy, 4, 2, if rng.chance(0.5) { b'@' } else { b'#' });
    }
    b
}

pub fn containers(rng: &mut Rng) -> Band {
    let mut b = band(ri(rng, 5, 7));
    for y in [1, 4] {
        let mut x = ri(rng, 0, 2);
        while y + 2 <= b.len() && x + 4 <= W {
            if rng.chance(0.7) {
                rect(&mut b, x, y, 4, 2, if rng.chance(0.5) { b'@' } else { b'#' });
            }
            x += 4 + ri(rng, 2, 3);
        }
    }
    b
}

pub fn cranes(rng: &mut Rng) -> Band {
    let mut b = band(6);
    for _ in 0..2 {
        let x = ri(rng, 0, W - 6);
        rect(&mut b, x, 1, 2, 2, b'@');
        rect(&mut b, x + 4, 1, 2, 2, b'@');
        rect(&mut b, x, 4, 6, 1, b'#');
    }
    b
}

pub fn quay(rng: &mut Rng) -> Band {
    let mut b = band(5);
    rect(&mut b, 0, 1, W, 1, b'#');
    river(&mut b, rng, 2, 2, 2);
    for _ in 0..ri(rng, 2, 4) {
        let x = ri(rng, 0, W - 1);
        set(&mut b, x, 4, b'@');
    }
    b
}

// --- Rail Yards: tracks with wagons, platforms, an engine shed, signal posts.

pub fn tracks(rng: &mut Rng) -> Band {
    let h = ri(rng, 8, 12);
    let mut b = band(h);
    let mut x = ri(rng, 0, 1);
    while x + 2 <= W {
        rect(&mut b, x, 0, 2, h, b'-');
        let mut y = ri(rng, 0, 3);
        while y + 3 <= h {
            let l = ri(rng, 3, 5).min(h - y);
            if rng.chance(0.7) {
                rect(&mut b, x, y, 2, l, b'#');
            }
            y += l + ri(rng, 2, 3);
        }
        x += 4 + ri(rng, 0, 1);
    }
    b
}

pub fn platform(rng: &mut Rng) -> Band {
    let mut b = band(4);
    let gaps = ri(rng, 3, 4);
    wall(&mut b, rng, 1, 2, b'#', gaps);
    let mut x = ri(rng, 0, 4);
    while x < W {
        set(&mut b, x, 3, b'@');
        x += 6;
    }
    b
}

pub fn depot(rng: &mut Rng) -> Band {
    let mut b = band(9);
    let w = ri(rng, 10, 14);
    let x = ri(rng, 0, W - w);
    rect(&mut b, x, 1, w, 7, b'#');
    rect(&mut b, x + 1, 2, w - 2, 5, b'-');
    for _ in 0..ri(rng, 2, 3) {
        let d = ri(rng, x + 1, x + w - 3);
        rect(&mut b, d, 1, 2, 1, b'.');
    }
    let d = ri(rng, x + 1, x + w - 3);
    rect(&mut b, d, 7, 2, 1, b'.');
    b
}

pub fn signals(rng: &mut Rng) -> Band {
    let mut b = band(ri(rng, 3, 4));
    rect(&mut b, 0, 1, W, 1, b'-');
    let n = ri(rng, 3, 5);
    scatter(&mut b, rng, n, 1, 1, b'@');
    b
}

// --- Citadel Gate: trenches, bunkers, tank traps, zigzag ramparts.

pub fn trench(rng: &mut Rng) -> Band {
    let mut b = band(5);
    let n = ri(rng, 2, 3);
    river(&mut b, rng, 2, 1, n);
    for x in 0..W {
        if rng.chance(0.5) {
            set(&mut b, x, 3, b'#');
        }
    }
    b
}

pub fn bunkers(rng: &mut Rng) -> Band {
    let mut b = band(7);
    for _ in 0..ri(rng, 2, 3) {
        let (x, y) = (ri(rng, 1, W - 5), ri(rng, 1, 2));
        rect(&mut b, x - 1, y, 4, 4, b'#');
        rect(&mut b, x, y + 1, 2, 2, b'@');
    }
    b
}

pub fn teeth(rng: &mut Rng) -> Band {
    let mut b = band(6);
    let ox = ri(rng, 0, 2);
    for (i, y) in [1, 4].into_iter().enumerate() {
        let mut x = (ox + i) % 3;
        while x < W {
            if rng.chance(0.7) {
                set(&mut b, x, y, b'@');
            }
            x += 3;
        }
    }
    b
}

pub fn ramparts(rng: &mut Rng) -> Band {
    let mut b = band(6);
    let (mut x, mut up) = (0, rng.chance(0.5));
    while x < W {
        let w = ri(rng, 4, 6).min(W - x);
        rect(&mut b, x, if up { 3 } else { 1 }, w, 2, b'#');
        x += w;
        up = !up;
    }
    for _ in 0..ri(rng, 1, 2) {
        let gx = ri(rng, 0, W - 3);
        rect(&mut b, gx, 1, 3, 4, b'.');
    }
    b
}

// --- The Citadel: steel courtyards, brick mazes, the moat, pillar halls.

pub fn courtyard(rng: &mut Rng) -> Band {
    let h = ri(rng, 8, 10);
    let mut b = band(h);
    let w = ri(rng, 10, 16);
    let x = ri(rng, 0, W - w);
    let top = h - 2;
    rect(&mut b, x, 1, w, top, b'@');
    rect(&mut b, x + 1, 2, w - 2, top - 2, b'.');
    let (d1, d2) = (ri(rng, x + 1, x + w - 4), ri(rng, x + 1, x + w - 4));
    rect(&mut b, d1, 1, 3, 1, b'.');
    rect(&mut b, d2, top, 3, 1, b'.');
    for _ in 0..ri(rng, 2, 3) {
        let (ix, iy) = (ri(rng, x + 2, x + w - 4), ri(rng, 3, top - 2));
        rect(&mut b, ix, iy, 2, 1, if rng.chance(0.5) { b'%' } else { b'#' });
    }
    b
}

pub fn maze(rng: &mut Rng) -> Band {
    let mut b = band(8);
    for y in [1, 4, 7] {
        let gaps = ri(rng, 2, 3);
        wall(&mut b, rng, y, 1, b'#', gaps);
    }
    for _ in 0..ri(rng, 2, 4) {
        let (x, y) = (ri(rng, 0, W - 1), ri(rng, 1, 5));
        rect(&mut b, x, y, 1, 3, b'#');
    }
    b
}

pub fn moat(rng: &mut Rng) -> Band {
    let mut b = band(5);
    for x in river(&mut b, rng, 1, 2, 2) {
        set(&mut b, x - 1, 3, b'@');
        set(&mut b, x + 3, 3, b'@');
    }
    b
}

pub fn pillars(rng: &mut Rng) -> Band {
    let mut b = band(6);
    let ox = ri(rng, 0, 3);
    for (i, y) in [1, 4].into_iter().enumerate() {
        let mut x = (ox + i * 2) % 5;
        while x + 2 <= W {
            rect(&mut b, x, y, 2, 2, b'@');
            x += 5;
        }
    }
    b
}
