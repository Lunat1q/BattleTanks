//! City Run district-map backdrop: a pixel-art night city seen from above. One canvas pixel per unit,
//! shown with nearest sampling so it stays chunky when stretched across a TV.

use super::canvas::*;
use bevy::image::ImageSampler;
use bevy::math::Vec2;
use bevy::prelude::Image;

/// Matches the map panel's 130:56 aspect.
pub const W: f32 = 232.0;
pub const H: f32 = 100.0;
/// City block in pixels (incl. its street on the left/bottom).
const BW: i32 = 12;
const BH: i32 = 10;

const GROUND: C = rgb(0.07, 0.075, 0.09);
const ROAD: C = rgb(0.15, 0.155, 0.17);
const AVENUE: C = rgb(0.19, 0.19, 0.21);
const WATER: C = rgb(0.06, 0.13, 0.24);
const RIPPLE: C = rgb(0.13, 0.25, 0.40);
const STONE: C = rgb(0.24, 0.22, 0.24);
const GRASS: C = rgb(0.06, 0.15, 0.08);
const TREE: C = rgb(0.10, 0.28, 0.12);
const LAMP: C = rgb(0.95, 0.74, 0.36);
const ROOFS: [C; 5] = [rgb(0.20, 0.22, 0.30), rgb(0.26, 0.24, 0.28), rgb(0.18, 0.24, 0.27), rgb(0.30, 0.22, 0.22), rgb(0.23, 0.23, 0.23)];

/// River centre line (pixel row) at column `x`: a lazy S climbing to the right.
fn river_y(x: f32) -> f32 {
    24.0 + 13.0 * (x * 0.034 + 0.8).sin() + x * 0.17
}

fn city_pixel(x: i32, y: i32) -> C {
    let (bx, by) = (x.div_euclid(BW), y.div_euclid(BH));
    let (fx, fy) = (x.rem_euclid(BW), y.rem_euclid(BH));
    let (av_v, av_h) = (bx % 4 == 0, by % 3 == 0);
    let (rw, rh) = (if av_v { 3 } else { 2 }, if av_h { 3 } else { 2 });
    let road = fx < rw || fy < rh;
    let d = (y as f32 - river_y(x as f32)).abs();
    if d < 4.5 {
        if road && av_v {
            // Bridge deck with railings.
            return if fx == 0 || fx == rw - 1 { STONE } else { AVENUE };
        }
        let ripple = hash(x / 3, y, 41) > 0.8 && (x + y) % 2 == 0;
        return if ripple { RIPPLE } else { WATER };
    }
    if d < 5.5 {
        return STONE;
    }
    if road {
        if fx == 0 && fy == 0 && hash(bx, by, 43) > 0.4 {
            return LAMP;
        }
        let dash = (av_v && fx == 1 && fy >= rh && y % 4 < 2) || (av_h && fy == 1 && fx >= rw && x % 4 < 2);
        return if dash {
            rgb(0.42, 0.38, 0.22)
        } else if av_v || av_h {
            AVENUE
        } else {
            ROAD
        };
    }
    let (ix, iy, iw, ih) = (fx - rw, fy - rh, BW - rw, BH - rh);
    let kind = hash(bx, by, 7);
    if kind < 0.14 {
        // Park: grass with clumps of trees, a shadow pixel under each.
        let t = hash(x / 2, y / 2, 45) > 0.55;
        let below = hash(x / 2, (y + 1) / 2, 45) > 0.55;
        return if t {
            TREE
        } else if below {
            rgb(0.04, 0.10, 0.05)
        } else {
            GRASS
        };
    }
    if kind < 0.2 {
        return if (ix + iy) % 2 == 0 { rgb(0.17, 0.16, 0.16) } else { rgb(0.14, 0.13, 0.14) }; // paved plaza
    }
    // Buildings: one or two lots per block, 1 px of ground between them.
    let split = hash(bx, by, 3) > 0.45;
    let (lot, lx, lw) = if split && ix >= iw / 2 {
        (1, ix - iw / 2, iw - iw / 2)
    } else if split {
        (0, ix, iw / 2)
    } else {
        (0, ix, iw)
    };
    if lx >= lw - 1 || iy >= ih - 1 {
        return GROUND;
    }
    let roof = ROOFS[(hash(bx * 2 + lot, by, 5) * ROOFS.len() as f32) as usize % ROOFS.len()];
    let (top, right) = (iy == ih - 2, lx == lw - 2);
    let (bottom, left) = (iy == 0, lx == 0);
    if top || left {
        return shade(roof, 1.35);
    }
    if bottom || right {
        return shade(roof, 0.6);
    }
    if hash(x, y, 47) > 0.93 {
        return LAMP; // a lit skylight
    }
    if hash(x, y, 48) > 0.9 {
        return shade(roof, 0.8); // roof clutter
    }
    roof
}

pub fn city_backdrop() -> Image {
    let mut c = Canvas::new(W, H, 1.0);
    // The citadel (district 10, at 90% / 14% of the panel) is a star fort over the grid.
    let fort = Vec2::new(W * 0.40, H * 0.36);
    let walls = star(fort, 15.0, 10.0, 5);
    c.fill(|p| {
        let (x, y) = ((p.x + W / 2.0).floor() as i32, (p.y + H / 2.0).floor() as i32);
        let s = sd_poly(p, &walls);
        let mut col = if s.abs() < 1.6 {
            if (x + y) % 3 == 0 { rgb(0.36, 0.30, 0.30) } else { rgb(0.30, 0.25, 0.26) }
        } else if s < 0.0 {
            rgb(0.11, 0.09, 0.10)
        } else if s < 3.0 {
            WATER // moat
        } else {
            city_pixel(x, y)
        };
        // Stepped vignette keeps the pixel look and lets the dots read on top.
        let e = ((p.x / (W / 2.0)).abs().max((p.y / (H / 2.0)).abs()) - 0.6).max(0.0) / 0.4;
        let k = 1.0 - ((e * e * 4.0).floor() / 4.0) * 0.5;
        col = shade(col, k * 0.85);
        col
    });
    let mut img = c.into_image();
    img.sampler = ImageSampler::nearest();
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backdrop_is_one_pixel_per_unit_and_opaque() {
        let img = city_backdrop();
        assert_eq!((img.width(), img.height()), (W as u32, H as u32));
        assert!(img.data.unwrap().chunks(4).all(|px| px[3] == 255));
    }
}
