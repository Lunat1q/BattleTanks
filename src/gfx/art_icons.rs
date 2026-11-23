//! Small art: bullets, particles, spawn sparkle, shield bubble, power-up icons, HUD flag.

use super::canvas::*;
use crate::rules::PowerKind;
use bevy::math::Vec2;
use bevy::prelude::Image;
use std::f32::consts::TAU;

/// White, tinted (HDR) by the sprite so it blooms.
pub fn bullet() -> Image {
    let mut c = Canvas::new(4.0, 6.0, 8.0);
    c.paint(|p| sd_seg(p, Vec2::new(0.0, -1.6), Vec2::new(0.0, 1.6), 1.2), |p| alpha(gray(1.0), 0.35 + 0.65 * (p.y + 2.0) / 4.0));
    c.paint(|p| sd_seg(p, Vec2::new(0.0, -0.8), Vec2::new(0.0, 1.8), 0.7), |_| gray(1.0));
    c.into_image()
}

/// Soft round dot for particles.
pub fn dot() -> Image {
    let mut c = Canvas::new(8.0, 8.0, 4.0);
    c.fill(|p| {
        let t = (1.0 - p.length() / 4.0).clamp(0.0, 1.0);
        alpha(gray(1.0), t * t)
    });
    c.into_image()
}

/// Four-point sparkle for spawning tanks.
pub fn sparkle() -> Image {
    let mut c = Canvas::new(16.0, 16.0, 8.0);
    let pts = star(Vec2::ZERO, 7.5, 1.2, 4);
    c.paint(|p| sd_circle(p, Vec2::ZERO, 3.5), |p| alpha(gray(1.0), (1.0 - p.length() / 3.5).max(0.0) * 0.6));
    c.paint(|p| sd_poly(p, &pts), |p| alpha(gray(1.0), 1.0 - (p.length() / 7.5) * 0.6));
    c.into_image()
}

/// Chunky pixel-art gold star (17 x 17 px, nearest sampling): the in-stage achievement flyout.
pub fn gold_star() -> Image {
    const N: f32 = 17.0;
    let pts = star(Vec2::new(0.0, -0.6), 8.4, 4.0, 5);
    let mut c = Canvas::new(N, N, 1.0);
    c.fill(|p| {
        let d = sd_poly(p, &pts);
        if d < -0.4 {
            // Light top-right, deep orange bottom-left, a white glint near the top.
            if (p - Vec2::new(-1.5, 2.5)).length() < 1.2 {
                rgb(1.0, 1.0, 0.9)
            } else if p.x + p.y > 2.0 {
                rgb(1.0, 0.88, 0.36)
            } else if p.x + p.y < -3.0 {
                rgb(0.90, 0.52, 0.10)
            } else {
                rgb(0.98, 0.72, 0.20)
            }
        } else if d < 0.6 {
            rgb(0.36, 0.18, 0.04)
        } else {
            [0.0; 4]
        }
    });
    let mut img = c.into_image();
    img.sampler = bevy::image::ImageSampler::nearest();
    img
}

/// Shield bubble ring.
pub fn ring() -> Image {
    let mut c = Canvas::new(22.0, 22.0, 8.0);
    c.fill(|p| {
        let d = (p.length() - 9.6).abs();
        let glow = (1.0 - d / 1.6).max(0.0);
        let hex = ((p.x * 1.2).sin() * (p.y * 1.2).sin()).abs() * 0.15 * (p.length() < 9.6) as u8 as f32 * 0.5;
        alpha(gray(1.0), glow * glow * 0.9 + hex)
    });
    c.into_image()
}

pub fn flag() -> Image {
    let mut c = Canvas::new(16.0, 16.0, 8.0);
    c.outlined(|p| sd_box(p, Vec2::new(-4.5, 0.0), Vec2::new(0.6, 7.0)), 0.3, |_| gray(0.85));
    let cloth = [Vec2::new(-3.9, 6.5), Vec2::new(6.0, 3.2), Vec2::new(-3.9, -0.2)];
    c.outlined(|p| sd_poly(p, &cloth), 0.3, |p| mix(rgb(0.95, 0.35, 0.25), rgb(0.75, 0.15, 0.12), (p.x + 4.0) / 10.0));
    c.into_image()
}

const POWERUP_PX: f32 = 48.0;

/// Power-up tile, 48 x 48 pixel art like the achievement badges (navy plate, gold rim, crisp outlined emblem), shown
/// with nearest sampling: 3 px per field unit.
pub fn powerup(kind: PowerKind) -> Image {
    use super::badge_art::{self as b, flat, toon, v};
    // Laid out in the badges' 32 units, painted at 48 px (owner: 32 px read too coarse on the field).
    let mut c = Canvas::new(32.0, 32.0, POWERUP_PX / 32.0);
    match kind {
        PowerKind::Helmet => {
            let at = v(0.0, -3.0);
            c.paint(|p| sd_circle(p, at, 10.0).max(at.y - p.y), toon(b::LIGHT, at + v(0.0, 4.0), 7.0));
            c.paint(|p| sd_box(p, at + v(0.0, 6.0), v(1.0, 3.5)).max(sd_circle(p, at, 10.0)), flat(b::STEEL));
            c.paint(|p| sd_rbox(p, at + v(0.0, -1.0), v(12.0, 2.0), 0.8), toon(b::STEEL, at, 2.0));
            b::star5(&mut c, at + v(0.0, 4.5), 3.8, b::GOLD);
        }
        PowerKind::Clock => {
            let at = v(0.0, -1.5);
            c.paint(|p| sd_box(p, at + v(0.0, 11.5), v(2.2, 1.4)), flat(b::GOLD));
            c.paint(|p| sd_circle(p, at, 10.0), flat(b::GOLD));
            c.paint(|p| sd_circle(p, at, 8.0), toon(b::WHITE, at, 8.0));
            for k in 0..4 {
                let d = Vec2::from_angle(k as f32 * TAU / 4.0);
                c.paint(|p| sd_box(p, at + d * 6.2, v(0.6, 0.6) + d.abs()), flat(b::DARK));
            }
            c.paint(|p| sd_seg(p, at, at + v(0.0, 5.5), 0.7), flat(b::DARK));
            c.paint(|p| sd_seg(p, at, at + v(4.0, 0.0), 0.7), flat(b::RED));
        }
        PowerKind::Shovel => {
            // Diagonal: T-grip top left, blade pointing bottom right; `uv` = along / across the shaft.
            let (d, n) = (v(1.0, -1.0).normalize(), v(1.0, 1.0).normalize());
            let uv = |u: f32, w: f32| v(-1.0, 1.0) + d * u + n * w;
            c.paint(|p| sd_seg(p, uv(-10.5, 0.0), uv(0.0, 0.0), 1.3), toon(b::WOOD, uv(-5.0, 0.0), 2.0));
            c.paint(|p| sd_seg(p, uv(-11.0, -3.2), uv(-11.0, 3.2), 1.3), flat(b::WOOD));
            let blade = [uv(0.0, -4.5), uv(7.0, -4.5), uv(12.0, 0.0), uv(7.0, 4.5), uv(0.0, 4.5)];
            c.paint(|p| sd_poly(p, &blade), toon(b::STEEL, uv(5.0, 0.0), 5.0));
            c.paint(|p| sd_seg(p, uv(1.0, 0.0), uv(7.5, 0.0), 0.6), flat(shade(b::STEEL, 0.65)));
        }
        PowerKind::Star => b::star5(&mut c, v(0.0, -0.8), 12.5, b::GOLD),
        PowerKind::Grenade => b::grenade(&mut c, v(0.0, -2.0)),
        PowerKind::Tank => b::tank(&mut c, v(0.0, 0.0), 19.0, rgb(0.42, 0.86, 0.48), true),
    }
    super::art_badges::tile(&c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powerup_tiles_are_48_px_and_all_differ() {
        let tiles: Vec<Vec<u8>> = PowerKind::ALL
            .iter()
            .map(|&k| {
                let img = powerup(k);
                assert_eq!((img.width(), img.height()), (48, 48));
                img.data.unwrap()
            })
            .collect();
        for (i, a) in tiles.iter().enumerate() {
            assert!(tiles[i + 1..].iter().all(|b| a != b), "tile {i} repeats");
        }
    }
}
