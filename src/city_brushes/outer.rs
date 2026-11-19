//! Brushes for the outer districts 1-5 (see `city_brushes`).

use super::{Band, W, band, patch, patches, rect, ri, river, scatter, set, wall};
use crate::rng::Rng;

// --- Outskirts: fields, hedgerows, farmsteads, ponds.

pub fn fields(rng: &mut Rng) -> Band {
    let mut b = band(ri(rng, 6, 9));
    let n = ri(rng, 2, 4);
    patches(&mut b, rng, n, (3, 7), (2, 4), b'%');
    b
}

pub fn hedgerow(rng: &mut Rng) -> Band {
    let mut b = band(4);
    let gaps = ri(rng, 2, 4);
    wall(&mut b, rng, 1, 1, b'#', gaps);
    let n = ri(rng, 2, 4);
    scatter(&mut b, rng, n, 2, 1, b'%');
    b
}

pub fn farm(rng: &mut Rng) -> Band {
    let mut b = band(7);
    let x = ri(rng, 1, W - 9);
    patch(&mut b, rng, x, 2, 5, 3, b'#');
    set(&mut b, x + 2, 2, b'.');
    rect(&mut b, x + 6, 2, 2, 2, b'@'); // silo
    let (pw, ph) = (ri(rng, 3, 5), ri(rng, 2, 3));
    let px = if x > W / 2 - 4 { ri(rng, 0, 3) } else { ri(rng, W - 4 - pw, W - pw) };
    patch(&mut b, rng, px, 1, pw, ph, b'~');
    b
}

pub fn pond(rng: &mut Rng) -> Band {
    let mut b = band(ri(rng, 5, 7));
    let (w, h) = (ri(rng, 4, 9), ri(rng, 2, 3));
    let x = ri(rng, 0, W - w);
    patch(&mut b, rng, x, 1, w, h, b'~');
    let n = ri(rng, 2, 4);
    scatter(&mut b, rng, n, 2, 1, b'%');
    b
}

pub fn lane(rng: &mut Rng) -> Band {
    let mut b = band(3);
    let n = ri(rng, 1, 3);
    scatter(&mut b, rng, n, 1, 1, b'%');
    b
}

// --- Market Row: stall rows, arcades, fountain squares, crates.

pub fn stalls(rng: &mut Rng) -> Band {
    let mut b = band(ri(rng, 6, 8));
    let w = ri(rng, 2, 3);
    let mut x = ri(rng, 0, 2);
    while x + w <= W {
        for y in [1, 5] {
            if y + 2 <= b.len() && rng.chance(0.8) {
                rect(&mut b, x, y, w, 2, b'#');
            }
        }
        x += w + 2;
    }
    b
}

pub fn arcade(rng: &mut Rng) -> Band {
    let mut b = band(5);
    rect(&mut b, 0, 1, W, 2, b'#');
    let step = ri(rng, 4, 6);
    let mut x = ri(rng, 0, 3);
    while x + 2 <= W {
        rect(&mut b, x, 1, 2, 2, b'.');
        x += step;
    }
    let n = ri(rng, 1, 3);
    for _ in 0..n {
        let x = ri(rng, 0, W - 1);
        set(&mut b, x, 3, b'@');
    }
    b
}

pub fn plaza(rng: &mut Rng) -> Band {
    let mut b = band(8);
    let cx = ri(rng, 4, W - 6);
    rect(&mut b, cx, 3, 2, 2, b'~');
    for (x, y) in [(cx - 1, 2), (cx + 2, 2), (cx - 1, 5), (cx + 2, 5)] {
        set(&mut b, x, y, b'@');
    }
    for _ in 0..ri(rng, 2, 4) {
        let (x, y) = (ri(rng, 0, W - 2), ri(rng, 0, 7));
        if x + 4 < cx || x > cx + 3 {
            rect(&mut b, x, y, 2, 1, b'%');
        }
    }
    b
}

pub fn crates(rng: &mut Rng) -> Band {
    let mut b = band(ri(rng, 4, 6));
    for _ in 0..ri(rng, 5, 9) {
        let w = ri(rng, 1, 2);
        let (x, y) = (ri(rng, 0, W - w), ri(rng, 0, b.len() - 1));
        rect(&mut b, x, y, w, 1, if rng.chance(0.2) { b'@' } else { b'#' });
    }
    b
}

// --- Canal Ward: canals with bridges and quays, a bend, terraced houses, locks.

pub fn canal(rng: &mut Rng) -> Band {
    let mut b = band(6);
    for y in [1, 4] {
        for x in 0..W {
            if rng.chance(0.6) {
                set(&mut b, x, y, b'#');
            }
        }
    }
    let n = ri(rng, 1, 3);
    for x in river(&mut b, rng, 2, 2, n) {
        rect(&mut b, x, 1, 3, 4, b'.');
        set(&mut b, x - 1, 4, b'@');
        set(&mut b, x + 3, 1, b'@');
    }
    b
}

pub fn bend(rng: &mut Rng) -> Band {
    let mut b = band(7);
    let s = ri(rng, 8, 18);
    rect(&mut b, 0, 1, s, 2, b'~');
    rect(&mut b, s - 2, 1, 2, 4, b'~');
    rect(&mut b, s - 2, 3, W - s + 2, 2, b'~');
    let (l, r) = (ri(rng, 0, s - 5), ri(rng, s, W - 3));
    rect(&mut b, l, 1, 3, 2, b'.');
    rect(&mut b, r, 3, 3, 2, b'.');
    let n = ri(rng, 2, 4);
    scatter(&mut b, rng, n, 1, 1, b'%');
    b
}

pub fn terrace(rng: &mut Rng) -> Band {
    let mut b = band(ri(rng, 5, 6));
    let mut x = ri(rng, 0, 2);
    while x + 2 <= W {
        if rng.chance(0.75) {
            let h = ri(rng, 2, 3);
            rect(&mut b, x, 1, 2, h, b'#');
            if rng.chance(0.4) {
                set(&mut b, x, 1 + h, b'%');
            }
        }
        x += ri(rng, 4, 5);
    }
    b
}

pub fn lock(rng: &mut Rng) -> Band {
    let mut b = band(7);
    let w = ri(rng, 3, 5);
    let x = ri(rng, 1, W - w - 1);
    rect(&mut b, x - 1, 0, w + 2, 7, b'#');
    rect(&mut b, x, 0, w, 7, b'~');
    rect(&mut b, x, 3, w, 1, b'@');
    let n = ri(rng, 2, 3);
    scatter(&mut b, rng, n, 2, 2, b'#');
    b
}

// --- Old Town: crooked house blocks, squares, a church, the old wall.

pub fn quarter(rng: &mut Rng) -> Band {
    let mut b = band(ri(rng, 8, 10));
    let mut x = ri(rng, 0, 2);
    while x + 3 <= W {
        let w = ri(rng, 3, 7).min(W - x);
        let h = ri(rng, 3, b.len() - 2);
        let y = ri(rng, 1, b.len() - h);
        patch(&mut b, rng, x, y, w, h, b'#');
        x += w + ri(rng, 2, 3);
    }
    b
}

pub fn square(rng: &mut Rng) -> Band {
    let mut b = band(7);
    let cx = ri(rng, 6, W - 8);
    rect(&mut b, cx, 3, 2, 2, b'@'); // statue
    for (x, y) in [(cx - 3, 1), (cx + 3, 1), (cx - 3, 5), (cx + 3, 5)] {
        rect(&mut b, x, y, 2, 1, b'%');
    }
    let (lw, rw) = (ri(rng, 0, 3), ri(rng, 0, 3));
    rect(&mut b, 0, 1, lw, 5, b'#');
    rect(&mut b, W - rw, 1, rw, 5, b'#');
    b
}

pub fn church(rng: &mut Rng) -> Band {
    let mut b = band(8);
    let x = ri(rng, 1, W - 8);
    rect(&mut b, x, 1, 6, 5, b'#');
    rect(&mut b, x + 2, 1, 2, 1, b'.');
    rect(&mut b, x + 2, 6, 2, 2, b'@'); // tower
    for _ in 0..ri(rng, 3, 5) {
        let (gx, gy) = (ri(rng, 0, W - 1), ri(rng, 0, 7));
        if gx + 1 < x || gx > x + 6 {
            set(&mut b, gx, gy, b'%');
        }
    }
    b
}

pub fn town_wall(rng: &mut Rng) -> Band {
    let mut b = band(5);
    let gaps = ri(rng, 2, 3);
    wall(&mut b, rng, 1, 2, b'#', gaps);
    let n = ri(rng, 1, 3);
    scatter(&mut b, rng, n, 2, 3, b'@');
    b
}

// --- Ironworks: factory halls, pipe runs, scrap yards, cooling ponds, conveyors.

pub fn hall(rng: &mut Rng) -> Band {
    let mut b = band(8);
    let (w, h) = (ri(rng, 8, 12), ri(rng, 4, 6));
    let x = ri(rng, 0, W - w);
    rect(&mut b, x, 1, w, h, b'#');
    rect(&mut b, x + 1, 2, w - 2, h - 2, b'.');
    for (cx, cy) in [(x, 1), (x + w - 1, 1), (x, h), (x + w - 1, h)] {
        set(&mut b, cx, cy, b'@');
    }
    let (d1, d2) = (ri(rng, x + 1, x + w - 3), ri(rng, x + 1, x + w - 3));
    rect(&mut b, d1, 1, 2, 1, b'.');
    rect(&mut b, d2, h, 2, 1, b'.');
    let (mx, my) = (ri(rng, x + 2, x + w - 3), ri(rng, 2, h - 1));
    set(&mut b, mx, my, b'@');
    b
}

pub fn pipes(rng: &mut Rng) -> Band {
    let mut b = band(4);
    let gaps = ri(rng, 2, 3);
    wall(&mut b, rng, 1, 1, b'@', gaps);
    for _ in 0..ri(rng, 2, 4) {
        let x = ri(rng, 0, W - 1);
        set(&mut b, x, 2, b'#');
        set(&mut b, x, 0, b'#');
    }
    b
}

pub fn yard(rng: &mut Rng) -> Band {
    let mut b = band(ri(rng, 5, 7));
    let (s, p) = (ri(rng, 2, 3), ri(rng, 4, 6));
    scatter(&mut b, rng, s, 2, 2, b'@');
    scatter(&mut b, rng, p, 2, 1, b'#');
    b
}

pub fn cooling(rng: &mut Rng) -> Band {
    let mut b = band(6);
    for _ in 0..ri(rng, 2, 3) {
        let x = ri(rng, 0, W - 5);
        rect(&mut b, x, 1, 5, 4, b'@');
        rect(&mut b, x + 1, 2, 3, 2, b'~');
    }
    b
}

pub fn conveyor(rng: &mut Rng) -> Band {
    let mut b = band(4);
    rect(&mut b, 0, 1, W, 2, b'-');
    let step = ri(rng, 5, 7);
    let mut x = ri(rng, 0, 3);
    while x < W {
        set(&mut b, x, 0, b'@');
        set(&mut b, x, 3, b'@');
        x += step;
    }
    b
}
