//! Terrain, ground and base art. Terrain textures are 16x16-unit tiles sampled per 4x4 sub-cell
//! (see `gfx::world`), so patterns stay continuous as bricks get chipped away.

use super::canvas::*;
use bevy::math::Vec2;
use bevy::prelude::Image;
use std::f32::consts::TAU;

pub const PPU: f32 = 8.0;
/// Tile textures cover this many units.
pub const TILE_UNITS: f32 = 16.0;

/// Tile-local position 0..16, y up.
fn local(p: Vec2) -> Vec2 {
    p + Vec2::splat(TILE_UNITS / 2.0)
}

pub fn brick() -> Image {
    let mut c = Canvas::new(TILE_UNITS, TILE_UNITS, PPU);
    c.fill(|p| brick_at(local(p)));
    c.into_image()
}

/// Brick wall colour at `q` (units, y up): 4 x 2 bricks in offset courses. Also builds the title's "BATTLE".
pub fn brick_at(q: Vec2) -> C {
    let mortar = rgb(0.36, 0.32, 0.29);
    let row = (q.y / 2.0).floor();
    let shift = if row as i32 % 2 == 0 { 0.0 } else { 2.0 };
    let col = ((q.x + shift) / 4.0).floor();
    let (bx, by) = (((q.x + shift) / 4.0).fract() * 4.0, (q.y / 2.0).fract() * 2.0);
    let m = 0.28;
    if bx < m || by < m {
        return shade(mortar, 0.9 + 0.2 * hash(q.x as i32 * 8, q.y as i32 * 8, 3));
    }
    let k = hash(col as i32, row as i32, 11);
    let base = mix(rgb(0.70, 0.30, 0.17), rgb(0.80, 0.42, 0.22), k);
    let grain = 0.9 + 0.2 * noise(q * 3.0, 5, 48);
    let edge = if by > 2.0 - 0.45 {
        1.18
    } else if by < m + 0.4 {
        0.78
    } else if bx > 4.0 - 0.4 {
        0.85
    } else {
        1.0
    };
    shade(base, grain * edge)
}

pub fn steel() -> Image {
    let mut c = Canvas::new(TILE_UNITS, TILE_UNITS, PPU);
    c.fill(|p| steel_at(local(p)));
    c.into_image()
}

/// Steel colour at `q` (units, y up): 8 x 8 bevelled plates with a raised centre. Also builds the title's "TANKS".
pub fn steel_at(q: Vec2) -> C {
    let face = rgb(0.58, 0.62, 0.68);
    let (x, y) = (q.x.rem_euclid(8.0), q.y.rem_euclid(8.0));
    let brushed = 0.94 + 0.08 * noise(Vec2::new(q.x * 6.0, q.y * 0.6), 9, 96);
    let b = 0.9; // bevel width
    if x < b || y > 8.0 - b {
        return shade(face, 1.35 * brushed);
    }
    if x > 8.0 - b || y < b {
        return shade(face, 0.55 * brushed);
    }
    // Raised centre plate.
    let d = (x - 4.0).abs().max((y - 4.0).abs());
    if d < 2.0 {
        return shade(face, 1.12 * brushed);
    }
    if d < 2.45 {
        return shade(face, if x - 4.0 > y - 4.0 { 0.72 } else { 1.25 });
    }
    shade(face, brushed)
}

/// Gold block at `q` (units, y up): 8 x 8 bevelled plates like steel, with a diagonal shine. Not a field tile; it
/// builds headings (achievements).
pub fn gold_at(q: Vec2) -> C {
    let face = rgb(0.96, 0.70, 0.22);
    let (x, y) = (q.x.rem_euclid(8.0), q.y.rem_euclid(8.0));
    let b = 0.9;
    if x < b || y > 8.0 - b {
        return shade(face, 1.3);
    }
    if x > 8.0 - b || y < b {
        return shade(face, 0.55);
    }
    let shine = ((x + y - 9.0).abs() < 1.1) as u8 as f32;
    let grain = 0.94 + 0.08 * noise(q * 2.0, 71, 64);
    mix(shade(face, grain * (0.9 + 0.04 * (x - y))), rgb(1.0, 0.97, 0.8), 0.55 * shine)
}

pub fn water(frame: u32) -> Image {
    let mut c = Canvas::new(TILE_UNITS, TILE_UNITS, PPU);
    let phase = frame as f32 * TAU / 3.0;
    c.fill(|p| water_at(local(p), phase));
    c.into_image()
}

/// Water colour at `q` (units, y up), the waves shifted by `phase` (radians, one loop = TAU).
pub fn water_at(q: Vec2, phase: f32) -> C {
    let w = (q.x * TAU / 8.0 + (q.y * TAU / 16.0).sin() * 1.6 + phase).sin();
    let w2 = (q.y * TAU / 4.0 + (q.x * TAU / 16.0).cos() * 2.0 - phase).sin();
    let deep = mix(rgb(0.05, 0.20, 0.45), rgb(0.10, 0.34, 0.64), 0.5 + 0.25 * w + 0.15 * w2);
    let crest = ((w * w2) - 0.55).max(0.0) * 2.2;
    mix(deep, rgb(0.75, 0.88, 1.0), crest)
}

pub fn trees() -> Image {
    let mut c = Canvas::new(TILE_UNITS, TILE_UNITS, PPU);
    c.fill(|p| trees_at(local(p)));
    c.into_image()
}

/// Canopy colour at `q` (units, y up): jittered leaf blobs on a 4-unit grid, repeating every 16 units.
pub fn trees_at(q: Vec2) -> C {
    let mut best: Option<(f32, Vec2, f32)> = None; // (distance - radius, offset, radius), topmost blob wins
    for i in 0..16 {
        let (gx, gy) = (i % 4, i / 4);
        let j = Vec2::new(hash(gx, gy, 21), hash(gx, gy, 22)) * 2.0 - Vec2::ONE;
        let (b, r) = (Vec2::new(gx as f32 * 4.0 + 2.0, gy as f32 * 4.0 + 2.0) + j * 0.9, 2.3 + hash(gx, gy, 23) * 0.8);
        let d = q - b;
        let d = Vec2::new((d.x + 8.0).rem_euclid(16.0) - 8.0, (d.y + 8.0).rem_euclid(16.0) - 8.0);
        let s = d.length() - r;
        if s < 0.0 && best.is_none_or(|(bs, _, _)| s < bs) {
            best = Some((s, d, r));
        }
    }
    match best {
        None => rgb(0.03, 0.12, 0.05),
        Some((_, d, r)) => {
            let light = 1.0 - ((d - Vec2::new(-0.8, 0.8)).length() / (r * 1.4)).clamp(0.0, 1.0);
            let leaf = 0.85 + 0.3 * noise(q * 2.5, 31, 40);
            shade(mix(rgb(0.08, 0.30, 0.10), rgb(0.32, 0.62, 0.22), light), leaf)
        }
    }
}

pub fn ice() -> Image {
    let mut c = Canvas::new(TILE_UNITS, TILE_UNITS, PPU);
    c.fill(|p| ice_at(local(p)));
    c.into_image()
}

/// Ice colour at `q` (units, y up): diagonal streaks, frost, the odd crack.
pub fn ice_at(q: Vec2) -> C {
    let streak = ((q.x + q.y) * TAU / 5.33).sin() * 0.5 + 0.5;
    let frost = noise(q * 1.5, 41, 24);
    let crack = ((q.x - q.y * 0.5) * TAU / 16.0 * 3.0).sin().abs() < 0.03;
    let base = mix(rgb(0.62, 0.76, 0.88), rgb(0.86, 0.94, 1.0), streak * 0.5 + frost * 0.4);
    if crack { shade(base, 0.8) } else { base }
}

/// Play-field floor (208x208 units) at a lower resolution.
pub fn ground() -> Image {
    let mut c = Canvas::new(208.0, 208.0, 4.0);
    c.fill(|p| {
        let q = p + Vec2::splat(104.0);
        let n = noise(q * 0.35, 51, 1 << 16) * 0.6 + noise(q * 1.7, 52, 1 << 16) * 0.4;
        let speck = if hash((q.x * 4.0) as i32, (q.y * 4.0) as i32, 53) > 0.985 { 0.05 } else { 0.0 };
        let grid = if (q.x.rem_euclid(16.0) < 0.25) || (q.y.rem_euclid(16.0) < 0.25) { 0.012 } else { 0.0 };
        let v = 0.085 + n * 0.05 + speck + grid;
        [v * 0.95, v, v * 1.08, 1.0]
    });
    c.into_image()
}

/// Headquarters emblem; `destroyed` draws rubble.
pub fn base(destroyed: bool) -> Image {
    let mut c = Canvas::new(16.0, 16.0, PPU);
    let plate = |p: Vec2| sd_rbox(p, Vec2::ZERO, Vec2::splat(7.4), 1.5);
    if destroyed {
        c.paint(plate, |p| shade(rgb(0.18, 0.16, 0.15), 0.8 + 0.4 * noise(p * 1.5, 61, 64)));
        for i in 0..9 {
            let ctr = Vec2::new(hash(i, 0, 62) * 10.0 - 5.0, hash(i, 1, 62) * 10.0 - 5.0);
            let r = 1.2 + hash(i, 2, 62) * 1.6;
            let pts: Vec<Vec2> =
                (0..5).map(|k| ctr + Vec2::from_angle(k as f32 * TAU / 5.0 + i as f32) * r * (0.6 + hash(i, k, 63) * 0.6)).collect();
            c.outlined(|p| sd_poly(p, &pts), 0.3, |_| gray(0.25 + hash(i, 9, 64) * 0.2));
        }
        return c.into_image();
    }
    c.outlined(plate, 0.5, |p| mix(rgb(0.20, 0.24, 0.32), rgb(0.32, 0.38, 0.50), 0.5 + p.y / 16.0));
    c.paint(|p| sd_circle(p, Vec2::ZERO, 5.6).abs() - 0.45, |_| rgb(0.93, 0.74, 0.30));
    let star_pts = star(Vec2::new(0.0, -0.2), 4.6, 1.9, 5);
    c.outlined(|p| sd_poly(p, &star_pts), 0.35, |p| mix(rgb(1.0, 0.86, 0.45), rgb(0.95, 0.60, 0.18), 0.5 - p.y / 8.0));
    c.into_image()
}
