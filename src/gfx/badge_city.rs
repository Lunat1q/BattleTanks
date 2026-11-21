//! City Run badge emblems: each district's clear shows what the district is known for (the scarecrow of the
//! Outskirts, the burning Citadel...), plus the City Run extras. Same canvas rules as `badge_art`.

use super::badge_art::*;
use super::canvas::*;
use bevy::math::Vec2;
use std::f32::consts::TAU;

const STONE: C = rgb(0.72, 0.68, 0.62);
const BRICK: C = rgb(0.74, 0.36, 0.22);
const IRON: C = rgb(0.40, 0.42, 0.48);
const ICE: C = rgb(0.74, 0.92, 1.0);
const GLOW: C = rgb(1.0, 0.62, 0.20);

fn bx(c: &mut Canvas, at: Vec2, half: Vec2, col: C) {
    c.paint(|p| sd_box(p, at, half), toon(col, at, half.x.max(half.y)));
}

fn crenels(c: &mut Canvas, xs: &[f32], y: f32, col: C) {
    for &x in xs {
        bx(c, v(x, y), v(1.0, 1.0), col);
    }
}

fn district(n: usize, c: &mut Canvas) {
    match n {
        // Outskirts: a scarecrow over the furrows, one confused crow.
        1 => {
            c.paint(|p| sd_box(p, v(0.0, -9.5), v(11.0, 2.0)), |p| if (p.y + 20.0) as i32 % 2 == 0 { GRASS } else { shade(GRASS, 0.7) });
            bx(c, v(0.0, -3.0), v(0.9, 6.5), WOOD);
            bx(c, v(0.0, 2.5), v(8.5, 0.9), WOOD);
            c.paint(
                |p| sd_poly(p, &[v(-4.0, 3.5), v(4.0, 3.5), v(3.0, -4.0), v(-3.0, -4.0)]),
                toon(rgb(0.30, 0.48, 0.82), v(0.0, 0.0), 4.0),
            );
            c.paint(|p| sd_box(p, v(1.5, -1.5), v(1.0, 1.0)), flat(rgb(0.86, 0.36, 0.30)));
            c.paint(|p| sd_circle(p, v(0.0, 6.5), 2.8), toon(rgb(0.96, 0.82, 0.40), v(0.0, 6.5), 2.5));
            c.paint(
                |p| sd_poly(p, &[v(-3.0, 8.5), v(3.0, 8.5), v(0.5, 12.5)]).min(sd_box(p, v(0.0, 8.6), v(4.6, 0.6))),
                flat(shade(WOOD, 0.75)),
            );
            c.paint(|p| sd_circle(p, v(8.0, 7.0), 1.7).min(sd_poly(p, &[v(9.0, 7.5), v(11.5, 6.8), v(9.0, 6.2)])), flat(DARK));
        }
        // Market Row: a striped stall with fruit on the counter.
        2 => {
            for x in [-8.0, 8.0] {
                bx(c, v(x, -2.0), v(0.8, 8.0), WOOD);
            }
            bx(c, v(0.0, -6.0), v(9.5, 3.5), WOOD);
            for (k, col) in [rgb(1.0, 0.6, 0.15), rgb(0.5, 0.86, 0.3), RED, rgb(1.0, 0.86, 0.3)].into_iter().enumerate() {
                let q = v(-6.0 + k as f32 * 4.0, -1.5);
                c.paint(|p| sd_circle(p, q, 1.8), toon(col, q, 1.5));
            }
            c.paint(
                |p| sd_poly(p, &[v(-11.0, 5.0), v(11.0, 5.0), v(8.0, 10.5), v(-8.0, 10.5)]),
                |p| {
                    if (p.x / 3.4 + 10.0) as i32 % 2 == 0 { RED } else { WHITE }
                },
            );
            for k in 0..6 {
                let q = v(-9.2 + k as f32 * 3.7, 5.0);
                c.paint(|p| sd_circle(p, q, 1.8).max(p.y - q.y), move |_| if k % 2 == 0 { RED } else { WHITE });
            }
        }
        // Canal Ward: a stone bridge over the water, a lock gate beside it.
        3 => {
            c.paint(|p| (p.y + 5.0 + (p.x * 0.8).sin() * 0.7).max(sd_box(p, v(0.0, -8.0), v(11.0, 5.0))), toon(WATER, v(0.0, -8.0), 4.0));
            c.paint(|p| sd_box(p, v(0.0, 0.0), v(11.0, 4.0)).max(-sd_circle(p, v(0.0, -6.5), 6.5)), toon(STONE, v(0.0, 0.0), 6.0));
            for x in [-10.0, -6.0, -2.0, 2.0, 6.0, 10.0] {
                bx(c, v(x, 5.5), v(0.6, 1.5), STONE);
            }
            bx(c, v(0.0, 7.0), v(11.0, 0.6), STONE);
            c.paint(|p| sd_box(p, v(-4.0, 0.5), v(0.5, 2.5)), flat(shade(STONE, 0.6)));
        }
        // Old Town: the church, with its brand-new skylight.
        4 => {
            bx(c, v(-3.0, -5.0), v(6.5, 5.5), STONE);
            c.paint(|p| sd_poly(p, &[v(-10.5, 0.5), v(4.5, 0.5), v(-3.0, 7.5)]), toon(rgb(0.70, 0.30, 0.22), v(-3.0, 3.0), 5.0));
            c.paint(|p| sd_circle(p, v(-5.0, 3.0), 1.6), flat(DARK));
            bx(c, v(7.0, -2.0), v(3.0, 8.5), STONE);
            c.paint(|p| sd_poly(p, &[v(3.8, 6.5), v(10.2, 6.5), v(7.0, 12.5)]), toon(IRON, v(7.0, 8.5), 3.0));
            c.paint(|p| sd_circle(p, v(7.0, 1.0), 1.7), flat(GOLD));
            c.paint(|p| sd_box(p, v(-3.0, -8.5), v(1.8, 2.0)).min(sd_circle(p, v(-3.0, -6.5), 1.8)), flat(DARK));
            c.paint(|p| sd_circle(p, v(-3.0, -3.0), 1.8), flat(GOLD));
        }
        // Ironworks: sawtooth halls, a smoking chimney, molten windows.
        5 => {
            bx(c, v(-3.0, -6.5), v(8.0, 4.5), BRICK);
            for k in 0..3 {
                let x = -11.0 + k as f32 * 5.4;
                c.paint(|p| sd_poly(p, &[v(x, -2.0), v(x, 2.5), v(x + 5.4, -2.0)]), toon(IRON, v(x + 2.0, 0.0), 3.0));
            }
            for x in [-8.0, -3.0, 2.0] {
                c.paint(|p| sd_box(p, v(x, -6.5), v(1.4, 1.6)), flat(GLOW));
            }
            bx(c, v(7.5, -2.0), v(2.2, 9.0), BRICK);
            c.paint(|p| sd_box(p, v(7.5, 5.0), v(2.6, 0.6)), flat(IRON));
            for (q, r) in [(v(7.0, 9.0), 2.2), (v(3.5, 10.5), 1.8), (v(10.0, 11.0), 1.4)] {
                c.paint(|p| sd_circle(p, q, r), toon(rgb(0.78, 0.78, 0.80), q, r));
            }
        }
        // Frost Park: a snowflake.
        6 => {
            for k in 0..3 {
                let d = Vec2::from_angle(k as f32 * TAU / 6.0 + TAU / 4.0);
                c.paint(|p| sd_seg(p, -d * 10.5, d * 10.5, 1.1), toon(ICE, Vec2::ZERO, 8.0));
            }
            for k in 0..6 {
                let d = Vec2::from_angle(k as f32 * TAU / 6.0 + TAU / 4.0);
                let at = d * 6.0;
                for s in [-1.0, 1.0] {
                    let tip = at + Vec2::from_angle(d.to_angle() + s * 0.9) * 3.6;
                    c.paint(|p| sd_seg(p, at, tip, 0.8), flat(ICE));
                }
            }
            c.paint(|p| sd_circle(p, Vec2::ZERO, 2.4), flat(WHITE));
        }
        // Harbour: an anchor.
        7 => {
            let blue = rgb(0.56, 0.66, 0.80);
            c.paint(|p| (sd_circle(p, v(0.0, 9.0), 2.5)).abs() - 1.0, flat(blue));
            bx(c, v(0.0, -0.5), v(1.3, 8.0), blue);
            bx(c, v(0.0, 4.5), v(5.5, 1.1), blue);
            c.paint(|p| ((p - v(0.0, 0.0)).length() - 8.5).abs().max(p.y + 1.5) - 1.4, toon(blue, v(0.0, -4.0), 8.0));
            for s in [-1.0, 1.0] {
                c.paint(|p| sd_poly(p, &[v(s * 6.5, -1.0), v(s * 11.0, -0.5), v(s * 9.5, -4.5)]), flat(blue));
            }
        }
        // Rail Yards: a locomotive on the line, letting off steam.
        8 => {
            bx(c, v(0.0, -9.5), v(11.0, 0.8), IRON);
            c.paint(|p| sd_rbox(p, v(-2.5, -1.5), v(7.0, 3.6), 1.5), toon(rgb(0.20, 0.44, 0.30), v(-2.5, -1.5), 5.0));
            bx(c, v(6.5, 1.0), v(3.5, 6.0), RED);
            c.paint(|p| sd_box(p, v(6.5, 3.5), v(2.0, 1.6)), flat(rgb(1.0, 0.86, 0.4)));
            bx(c, v(-7.0, 4.5), v(1.4, 2.5), DARK);
            c.paint(|p| sd_poly(p, &[v(-9.5, -5.0), v(-12.0, -8.5), v(-9.5, -8.5)]), flat(IRON));
            for x in [-6.5, -1.5, 5.0] {
                c.paint(|p| sd_circle(p, v(x, -6.5), 2.6), toon(RED, v(x, -6.5), 2.0));
                c.paint(|p| sd_circle(p, v(x, -6.5), 0.8), flat(DARK));
            }
            c.paint(|p| sd_circle(p, v(-8.0, 9.5), 2.4).min(sd_circle(p, v(-4.5, 10.5), 1.8)), toon(WHITE, v(-7.0, 10.0), 2.0));
        }
        // Citadel Gate: the gatehouse with its portcullis down.
        9 => {
            bx(c, v(0.0, -2.0), v(10.0, 9.0), STONE);
            crenels(c, &[-9.0, -4.5, 0.0, 4.5, 9.0], 8.0, STONE);
            let arch = |p: Vec2| sd_box(p, v(0.0, -6.5), v(5.0, 4.5)).min(sd_circle(p, v(0.0, -2.0), 5.0));
            c.paint(arch, flat(DARK));
            c.paint(|p| arch(p).max(((p.x + 20.0) % 2.5 - 1.25).abs() - 0.45), flat(IRON));
            c.paint(|p| arch(p).max(((p.y + 20.0) % 3.0 - 1.5).abs() - 0.45), flat(IRON));
            for x in [-7.5, 7.5] {
                c.paint(|p| sd_box(p, v(x, 1.0), v(0.7, 1.4)), flat(GLOW));
            }
        }
        // The Citadel: fallen, and still on fire.
        _ => {
            let stone = shade(STONE, 0.8);
            for x in [-7.5, 7.5] {
                bx(c, v(x, -3.0), v(3.0, 8.5), stone);
                crenels(c, &[x - 2.0, x + 2.0], 6.5, stone);
            }
            bx(c, v(0.0, -5.0), v(5.0, 6.5), stone);
            crenels(c, &[-3.5, 0.0, 3.5], 2.5, stone);
            c.paint(|p| sd_box(p, v(0.0, -9.0), v(2.0, 2.5)).min(sd_circle(p, v(0.0, -6.5), 2.0)), flat(DARK));
            c.paint(|p| sd_box(p, v(0.0, 6.5), v(0.4, 4.0)), flat(rgb(0.86, 0.86, 0.86)));
            c.paint(|p| sd_poly(p, &[v(0.4, 10.5), v(6.0, 9.0), v(0.4, 7.5)]), flat(GOLD));
            for x in [-7.5, 7.5] {
                flame(c, v(x, 10.0), 4.2, [RED, rgb(1.0, 0.56, 0.14), rgb(1.0, 0.9, 0.4)]);
            }
        }
    }
}

pub fn draw(key: &str, c: &mut Canvas) -> &'static str {
    if let Some(n) = key.strip_prefix("city_").and_then(|n| n.parse::<usize>().ok()) {
        district(n, c);
        return ["", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10"][n.min(10)];
    }
    match key {
        "untouchable" => {
            shield(c, v(0.0, 0.0), 7.5, GOLD);
            // Shells bouncing off, each with a spark where it struck.
            for q in [v(-10.5, 10.0), v(10.5, 3.0), v(-10.0, -4.0)] {
                c.paint(|p| sd_poly(p, &star(q, 2.6, 0.8, 4)), flat(rgb(1.0, 0.94, 0.55)));
            }
            ""
        }
        "ghost" => {
            let body = |p: Vec2| {
                let b = sd_circle(p, v(0.0, 3.0), 7.5).min(sd_box(p, v(0.0, -2.5), v(7.5, 5.5)));
                let wave = [-5.0, 0.0, 5.0].iter().map(|&x| sd_circle(p, v(x, -8.0), 2.2)).fold(f32::MAX, f32::min);
                b.max(-wave)
            };
            c.paint(body, toon(rgb(0.86, 0.93, 1.0), v(0.0, 2.0), 7.0));
            for x in [-3.0, 3.0] {
                c.paint(|p| sd_circle(p * v(1.0, 0.7), v(x, 3.0 * 0.7), 1.6), flat(DARK));
            }
            c.paint(|p| sd_circle(p, v(0.0, -1.5), 1.2), flat(DARK));
            ""
        }
        "digger" => {
            c.paint(
                |p| sd_box(p, Vec2::ZERO, v(9.0, 9.0)),
                |p| {
                    let row = ((p.y + 9.0) / 4.5).floor() as i32;
                    let x = p.x + 9.0 + if row % 2 == 0 { 0.0 } else { 4.5 };
                    let mortar = (p.y + 9.0) % 4.5 < 0.9 || x % 9.0 < 0.9;
                    if mortar { rgb(0.34, 0.20, 0.14) } else { BRICK }
                },
            );
            c.paint(|p| sd_circle(p, Vec2::ZERO, 5.6), flat(rgb(0.16, 0.10, 0.08)));
            star5(c, v(0.0, -0.3), 4.6, GOLD);
            ""
        }
        "treasure_hunter" => {
            c.paint(|p| sd_rbox(p, v(0.0, 4.0), v(9.0, 3.5), 2.5), toon(shade(WOOD, 0.85), v(0.0, 4.0), 5.0));
            c.paint(|p| sd_box(p, v(0.0, -3.5), v(9.0, 5.0)), toon(WOOD, v(0.0, -3.5), 6.0));
            for x in [-5.5, 5.5] {
                c.paint(|p| sd_box(p, v(x, 0.0), v(1.0, 8.5)), toon(GOLD, v(x, 0.0), 2.0));
            }
            c.paint(|p| sd_box(p, v(0.0, 0.5), v(2.0, 2.4)), toon(GOLD, v(0.0, 0.5), 2.0));
            c.paint(|p| sd_box(p, v(0.0, 0.0), v(0.5, 0.9)), flat(DARK));
            c.paint(|p| sd_poly(p, &star(v(-8.0, 10.0), 2.6, 0.8, 4)), flat(WHITE));
            "3"
        }
        "boss_slayer" => {
            let pts = [v(-9.0, -3.0), v(-9.0, 7.0), v(-4.5, 1.5), v(0.0, 9.0), v(4.5, 1.5), v(9.0, 7.0), v(9.0, -3.0)];
            c.paint(|p| sd_poly(p, &pts), toon(GOLD, v(0.0, 2.0), 8.0));
            c.paint(|p| sd_box(p, v(0.0, -5.0), v(9.0, 2.5)), toon(shade(GOLD, 0.85), v(0.0, -5.0), 5.0));
            for q in [v(-9.0, 7.5), v(0.0, 9.5), v(9.0, 7.5)] {
                c.paint(|p| sd_circle(p, q, 1.5), flat(WHITE));
            }
            for (x, col) in [(-5.0, RED), (0.0, rgb(0.3, 0.6, 1.0)), (5.0, RED)] {
                c.paint(|p| sd_circle(p, v(x, -5.0), 1.4), flat(col));
            }
            "10"
        }
        "wallbreaker" => {
            c.paint(
                |p| sd_box(p, v(0.0, -6.5), v(11.0, 4.5)).max(p.y + 2.0 + (p.x * 1.3).sin().abs() * 3.0 - 3.0),
                |p| {
                    let row = ((p.y + 11.0) / 3.0).floor() as i32;
                    let x = p.x + 11.0 + if row % 2 == 0 { 0.0 } else { 3.0 };
                    if (p.y + 11.0) % 3.0 < 0.8 || x % 6.0 < 0.8 { rgb(0.34, 0.20, 0.14) } else { BRICK }
                },
            );
            c.paint(|p| sd_box(p, v(0.0, 3.0), v(1.2, 7.5)), flat(IRON));
            c.paint(|p| sd_circle(p, v(0.0, 1.0), 4.8), toon(STEEL, v(0.0, 1.0), 4.0));
            for q in [v(-8.0, 6.0), v(7.0, 8.0), v(9.0, 3.0)] {
                c.paint(|p| sd_box(p, q, v(1.2, 0.8)), flat(BRICK));
            }
            "100"
        }
        _ => {
            debug_assert!(false, "no badge for {key}");
            question(c);
            ""
        }
    }
}
