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

const INK: C = rgb(0.98, 0.93, 0.78);

pub fn powerup(kind: PowerKind) -> Image {
    let mut c = Canvas::new(16.0, 16.0, 8.0);
    let frame = |p: Vec2| sd_rbox(p, Vec2::ZERO, Vec2::splat(7.6), 2.2);
    c.paint(|p| frame(p) - 0.6, |_| rgb(0.95, 0.80, 0.35));
    c.paint(frame, |p| mix(rgb(0.10, 0.14, 0.30), rgb(0.22, 0.30, 0.58), 0.5 + p.y / 16.0));
    let o = 0.45;
    match kind {
        PowerKind::Helmet => {
            c.outlined(|p| sd_circle(p, Vec2::new(0.0, -1.0), 4.8).max(-p.y - 1.0), o, |p| mix(INK, gray(0.7), -p.x / 5.0));
            c.outlined(|p| sd_rbox(p, Vec2::new(0.0, -1.6), Vec2::new(5.8, 0.8), 0.4), o, |_| gray(0.8));
        }
        PowerKind::Clock => {
            c.outlined(|p| sd_circle(p, Vec2::ZERO, 5.2), o, |_| INK);
            c.paint(|p| sd_seg(p, Vec2::ZERO, Vec2::new(0.0, 3.6), 0.5), |_| gray(0.15));
            c.paint(|p| sd_seg(p, Vec2::ZERO, Vec2::new(2.6, 0.0), 0.5), |_| gray(0.15));
            for k in 0..12 {
                let a = k as f32 * TAU / 12.0;
                c.paint(|p| sd_circle(p, Vec2::from_angle(a) * 4.3, 0.3), |_| gray(0.3));
            }
        }
        PowerKind::Shovel => {
            c.outlined(|p| sd_seg(p, Vec2::new(-4.0, 4.0), Vec2::new(1.0, -1.0), 0.6), o, |_| rgb(0.75, 0.55, 0.32));
            c.outlined(|p| sd_box(p, Vec2::new(-4.2, 4.2), Vec2::new(1.6, 0.6)), o, |_| rgb(0.75, 0.55, 0.32));
            let blade = [Vec2::new(0.0, 0.0), Vec2::new(2.6, -2.6), Vec2::new(5.0, -5.6), Vec2::new(2.2, -5.0), Vec2::new(-0.4, -2.4)];
            c.outlined(|p| sd_poly(p, &blade) - 0.6, o, |_| INK);
        }
        PowerKind::Star => {
            let pts = star(Vec2::new(0.0, -0.3), 6.0, 2.5, 5);
            c.outlined(|p| sd_poly(p, &pts), o, |p| mix(rgb(1.0, 0.92, 0.5), rgb(1.0, 0.65, 0.2), 0.5 - p.y / 10.0));
        }
        PowerKind::Grenade => {
            c.outlined(
                |p| sd_circle(p, Vec2::new(0.0, -1.2), 4.4),
                o,
                |p| mix(rgb(0.45, 0.6, 0.3), rgb(0.25, 0.35, 0.15), p.x / 8.0 + 0.5),
            );
            c.outlined(|p| sd_box(p, Vec2::new(0.0, 3.6), Vec2::new(1.5, 1.0)), o, |_| gray(0.7));
            c.paint(|p| sd_circle(p, Vec2::new(2.6, 4.6), 1.2).abs() - 0.35, |_| INK);
        }
        PowerKind::Tank => {
            c.outlined(|p| sd_rbox(p, Vec2::new(0.0, -1.5), Vec2::new(5.5, 2.5), 1.0), o, |_| INK);
            c.outlined(|p| sd_circle(p, Vec2::new(-0.5, 1.6), 2.2), o, |_| INK);
            c.outlined(|p| sd_box(p, Vec2::new(3.2, 2.0), Vec2::new(3.0, 0.5)), o, |_| INK);
            for k in 0..4 {
                c.paint(|p| sd_circle(p, Vec2::new(-3.6 + k as f32 * 2.4, -2.0), 0.7), |_| gray(0.3));
            }
        }
    }
    c.into_image()
}
