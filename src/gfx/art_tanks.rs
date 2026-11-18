//! Tank sprites, drawn in grey facing up and tinted per team/type by `Sprite::color`.

use super::canvas::*;
use crate::rules::EnemyKind;
use bevy::math::Vec2;
use bevy::prelude::Image;

pub const PPU: f32 = 8.0;

/// Shape parameters of one tank model (units, tank is 16x16).
#[derive(Clone, Copy)]
struct Look {
    hull: Vec2,
    tread_w: f32,
    turret: f32,
    barrel_len: f32,
    barrel_w: f32,
    twin: bool,
    plates: bool,
    nose: bool,
}

const BASE: Look =
    Look { hull: Vec2::new(4.4, 6.0), tread_w: 3.2, turret: 3.1, barrel_len: 7.4, barrel_w: 0.85, twin: false, plates: false, nose: false };

fn player_look(level: u8) -> Look {
    match level {
        0 => BASE,
        1 => Look { barrel_len: 8.0, barrel_w: 0.95, ..BASE },
        2 => Look { barrel_len: 8.0, barrel_w: 0.7, twin: true, turret: 3.3, ..BASE },
        _ => Look { barrel_len: 8.0, barrel_w: 1.15, turret: 3.6, plates: true, hull: Vec2::new(4.7, 6.3), ..BASE },
    }
}

fn enemy_look(kind: EnemyKind) -> Look {
    match kind {
        EnemyKind::Basic => Look { hull: Vec2::new(4.0, 5.4), tread_w: 3.0, turret: 2.8, barrel_len: 6.6, barrel_w: 0.8, ..BASE },
        EnemyKind::Fast => {
            Look { hull: Vec2::new(3.5, 6.6), tread_w: 2.6, turret: 2.4, barrel_len: 7.2, barrel_w: 0.7, nose: true, ..BASE }
        }
        EnemyKind::Power => Look { hull: Vec2::new(4.6, 5.8), turret: 3.5, barrel_len: 8.0, barrel_w: 1.25, ..BASE },
        EnemyKind::Armor => {
            Look { hull: Vec2::new(5.0, 6.5), tread_w: 3.0, turret: 3.7, barrel_len: 7.0, barrel_w: 1.05, plates: true, ..BASE }
        }
    }
}

fn draw(l: Look, frame: u32) -> Image {
    paint(l, frame, PPU, 16.0).into_image()
}

/// Grey tank (16 units) centred on a `span`-unit canvas at `ppu` pixels per unit.
fn paint(l: Look, frame: u32, ppu: f32, span: f32) -> Canvas {
    let mut c = Canvas::new(span, span, ppu);
    let tw = l.tread_w;
    // Treads with moving links.
    for side in [-1.0f32, 1.0] {
        let center = Vec2::new(side * (8.0 - tw / 2.0 - 0.2), 0.0);
        let half = Vec2::new(tw / 2.0, 7.5);
        c.outlined(
            |p| sd_rbox(p, center, half, 1.0),
            0.35,
            |p| {
                let link = ((p.y + frame as f32 * 0.75) / 1.5).rem_euclid(1.0);
                let edge = ((p.x - center.x).abs() / (tw / 2.0)).powi(2);
                gray(if link < 0.35 { 0.16 } else { 0.34 } - edge * 0.08)
            },
        );
    }
    // Hull.
    let hc = Vec2::new(0.0, -0.3);
    let hull = move |p: Vec2| {
        let body = sd_rbox(p, hc, l.hull, 1.2);
        if l.nose {
            let wedge = sd_poly(p, &[Vec2::new(-l.hull.x, 2.0), Vec2::new(0.0, l.hull.y + 1.2), Vec2::new(l.hull.x, 2.0)]);
            body.min(wedge)
        } else {
            body
        }
    };
    c.outlined(hull, 0.45, |p| {
        let t = (p - hc) / l.hull;
        let bevel = (-hull(p) / 1.0).clamp(0.0, 1.0); // 0 at edge -> 1 inside
        gray(0.62 + 0.16 * t.y - 0.08 * t.x + 0.12 * (1.0 - bevel) * (t.y - t.x).signum())
    });
    // Engine grille at the back.
    for k in 0..3 {
        let y = hc.y - l.hull.y + 1.0 + k as f32 * 0.8;
        c.paint(|p| sd_box(p, Vec2::new(0.0, y), Vec2::new(l.hull.x * 0.55, 0.18)), |_| gray(0.3));
    }
    if l.plates {
        for side in [-1.0f32, 1.0] {
            let pc = Vec2::new(side * (l.hull.x - 1.0), hc.y + 1.0);
            c.outlined(|p| sd_rbox(p, pc, Vec2::new(0.8, l.hull.y * 0.6), 0.4), 0.25, |p| gray(0.8 + 0.1 * (p.y - pc.y) / l.hull.y));
        }
    }
    // Barrel(s).
    let xs: &[f32] = if l.twin { &[-0.95, 0.95] } else { &[0.0] };
    for &x in xs {
        let (a, b) = (Vec2::new(x, 0.0), Vec2::new(x, l.barrel_len - 0.4));
        c.outlined(
            |p| sd_box(p, (a + b) / 2.0, Vec2::new(l.barrel_w / 2.0, (b.y - a.y) / 2.0)),
            0.3,
            |p| gray(0.55 + 0.25 * (1.0 - ((p.x - x) / (l.barrel_w / 2.0)).abs())),
        );
        c.outlined(|p| sd_box(p, Vec2::new(x, b.y - 0.4), Vec2::new(l.barrel_w / 2.0 + 0.25, 0.45)), 0.25, |_| gray(0.5));
    }
    // Turret with soft shadow and highlight.
    let tc = Vec2::new(0.0, -0.4);
    c.paint(|p| sd_circle(p, tc + Vec2::new(0.4, -0.5), l.turret + 0.3), |_| [0.0, 0.0, 0.0, 0.35]);
    c.outlined(
        |p| sd_circle(p, tc, l.turret),
        0.4,
        |p| {
            let h = 1.0 - (p.distance(tc + Vec2::new(-0.9, 0.9)) / (l.turret * 1.6)).clamp(0.0, 1.0);
            gray(0.6 + 0.38 * h * h)
        },
    );
    c.paint(|p| sd_circle(p, tc + Vec2::new(0.9, -0.9), l.turret * 0.3), |_| gray(0.42));
    c
}

/// `[level][frame]` player tank images.
pub fn players() -> Vec<[Image; 2]> {
    (0..4).map(|lv| [draw(player_look(lv), 0), draw(player_look(lv), 1)]).collect()
}

/// `[kind][frame]` enemy tank images.
pub fn enemies() -> Vec<[Image; 2]> {
    EnemyKind::ALL.iter().map(|&k| [draw(enemy_look(k), 0), draw(enemy_look(k), 1)]).collect()
}

/// Top player tank model (star level 3), grey, for the app icon.
pub fn hero(ppu: f32, span: f32) -> Canvas {
    paint(player_look(3), 0, ppu, span)
}
