//! Badge emblems, one per achievement, painted on a 32x32 canvas (1 unit = 1 pixel, origin in the middle, y up).
//! Shapes are lit from the top left in three flat bands, pixel-art style; `art_badges` makes them crisp, outlines
//! them and lays them on the plate. Returns the number tag (bottom-right) or "". City Run lives in `badge_city`.

use super::badge_city;
use super::canvas::*;
use bevy::math::Vec2;

pub fn v(x: f32, y: f32) -> Vec2 {
    Vec2::new(x, y)
}

pub const GOLD: C = rgb(1.0, 0.80, 0.30);
pub const RED: C = rgb(0.90, 0.22, 0.18);
pub const WHITE: C = rgb(0.95, 0.95, 0.92);
pub const STEEL: C = rgb(0.62, 0.67, 0.74);
pub const WOOD: C = rgb(0.62, 0.38, 0.20);
pub const DARK: C = rgb(0.10, 0.10, 0.13);
pub const WATER: C = rgb(0.22, 0.46, 0.86);
pub const GRASS: C = rgb(0.30, 0.62, 0.24);
const P1: C = rgb(1.0, 0.80, 0.32);
const P2: C = rgb(0.42, 0.92, 0.52);
const FIRE: C = rgb(1.0, 0.56, 0.14);

/// Flat colour lit from the top left: a light band, the colour, a dark band, around `at` over radius `r`.
pub fn toon(base: C, at: Vec2, r: f32) -> impl Fn(Vec2) -> C {
    move |p| {
        let d = (p - at).dot(v(-0.55, 0.83)) / r.max(1.0);
        shade(
            base,
            if d > 0.4 {
                1.28
            } else if d < -0.45 {
                0.70
            } else {
                1.0
            },
        )
    }
}

pub fn flat(c: C) -> impl Fn(Vec2) -> C {
    move |_| c
}

/// Top-down tank `s` pixels long, drawn for badge size (the game's sprites blur this small): striped treads,
/// a hull with a gap to the treads so the outline separates them, turret and barrel (`up`, else facing down).
pub fn tank(c: &mut Canvas, at: Vec2, s: f32, col: C, up: bool) {
    let (h, d) = (s / 2.0, if up { 1.0 } else { -1.0 });
    let (tread, link) = (shade(col, 0.38), shade(col, 0.68));
    for x in [-1.0, 1.0] {
        let tc = at + v(x * h * 0.76, -d * h * 0.05);
        c.paint(|p| sd_box(p, tc, v(h * 0.24, h * 0.86)), move |p| if (p.y - tc.y + 40.5) as i32 % 2 == 0 { tread } else { link });
    }
    let hull = move |p: Vec2| sd_box(p, at - v(0.0, d * h * 0.05), v(h * 0.5, h * 0.7));
    let barrel = move |p: Vec2| sd_box(p, at + v(0.0, d * h * 0.62), v((h * 0.1).max(0.75), h * 0.55));
    let turret = move |p: Vec2| sd_circle(p, at, h * 0.42);
    // Dark seams first, so hull, barrel and turret read apart from the treads and each other.
    c.paint(|p| hull(p) - 0.9, flat(DARK));
    c.paint(hull, toon(col, at, h));
    c.paint(|p| barrel(p).min(turret(p)) - 0.9, flat(DARK));
    c.paint(barrel, flat(shade(col, 0.85)));
    c.paint(turret, toon(shade(col, 1.12), at, h * 0.42));
}

pub const LIGHT: C = rgb(0.80, 0.82, 0.86);

pub fn burst(c: &mut Canvas, at: Vec2, r: f32, outer: C, inner: C) {
    let (a, b) = (star(at, r, r * 0.45, 9), star(at, r * 0.62, r * 0.3, 9));
    c.paint(|p| sd_poly(p, &a), flat(outer));
    c.paint(|p| sd_poly(p, &b), flat(inner));
}

pub fn star5(c: &mut Canvas, at: Vec2, r: f32, col: C) {
    let pts = star(at, r, r * 0.45, 5);
    c.paint(|p| sd_poly(p, &pts), toon(col, at, r));
}

/// Shield, `w` half-wide, point at the bottom.
pub fn shield(c: &mut Canvas, at: Vec2, w: f32, col: C) {
    let pts = [v(-w, w * 1.1), v(w, w * 1.1), v(w, 0.0), v(0.0, -w * 1.25), v(-w, 0.0)].map(|q| q + at);
    c.paint(|p| sd_poly(p, &pts) - 0.6, toon(col, at, w));
    let inner = pts.map(|q| at + (q - at) * 0.62);
    c.paint(|p| sd_poly(p, &inner).abs() - 0.5, flat(shade(col, 0.6)));
}

/// Waving flag on a pole planted in a grass mound.
pub fn flag(c: &mut Canvas, cloth: C) {
    c.paint(|p| sd_rbox(p, v(-6.0, -10.5), v(5.0, 1.6), 1.0), toon(GRASS, v(-6.0, -10.0), 4.0));
    c.paint(|p| sd_box(p, v(-6.0, 0.0), v(0.9, 10.5)), toon(rgb(0.86, 0.80, 0.70), v(-6.0, 0.0), 1.0));
    c.paint(|p| sd_circle(p, v(-6.0, 11.0), 1.6), flat(GOLD));
    let pts = [v(-5.0, 10.0), v(1.0, 11.5), v(9.5, 9.0), v(9.5, 1.0), v(2.0, 3.0), v(-5.0, 2.0)];
    c.paint(|p| sd_poly(p, &pts), toon(cloth, v(2.0, 6.0), 5.0));
}

pub fn grenade(c: &mut Canvas, at: Vec2) {
    let olive = rgb(0.40, 0.58, 0.26);
    c.paint(|p| sd_circle(p, at, 7.0), toon(olive, at, 6.0));
    for k in [-1.0, 1.0] {
        c.paint(|p| sd_box(p, at + v(k * 2.6, 0.0), v(0.5, 6.0)).max(sd_circle(p, at, 6.4)), flat(shade(olive, 0.55)));
    }
    c.paint(|p| sd_box(p, at + v(0.0, 2.5), v(6.4, 0.5)).max(sd_circle(p, at, 6.4)), flat(shade(olive, 0.55)));
    c.paint(|p| sd_box(p, at + v(0.0, 8.0), v(2.6, 1.8)), toon(STEEL, at + v(0.0, 8.0), 2.0));
    c.paint(|p| (sd_circle(p, at + v(4.6, 9.4), 2.2)).abs() - 0.6, flat(GOLD));
}

/// Flame: three nested tongues, outer to inner colours.
pub fn flame(c: &mut Canvas, at: Vec2, s: f32, cols: [C; 3]) {
    let shape = [
        (0.0, 1.0),
        (0.38, 0.42),
        (0.6, -0.05),
        (0.52, -0.42),
        (0.26, -0.66),
        (0.0, -0.7),
        (-0.26, -0.66),
        (-0.52, -0.42),
        (-0.6, -0.05),
        (-0.36, 0.3),
        (-0.12, 0.58),
    ];
    for (k, col) in [1.0, 0.68, 0.38].into_iter().zip(cols) {
        let pts: Vec<Vec2> = shape.iter().map(|&(x, y)| at + v(x, y - (1.0 - k) * 0.55) * s * k).collect();
        c.paint(|p| sd_poly(p, &pts), flat(col));
    }
}

/// A stack of `n` gold coins seen from the side, bottom at `at`.
pub fn coins(c: &mut Canvas, at: Vec2, n: usize) {
    for k in 0..n {
        let y = at.y + k as f32 * 3.2;
        c.paint(|p| sd_rbox(p, v(at.x, y), v(4.6, 1.5), 0.8), flat(shade(GOLD, 0.78)));
        c.paint(|p| sd_box(p, v(at.x, y + 1.0), v(3.8, 0.5)), flat(shade(GOLD, 1.25)));
    }
}

pub fn question(c: &mut Canvas) {
    let col = rgb(0.80, 0.82, 0.90);
    // The hook: a ring with its lower-left quarter cut away.
    c.paint(|p| (((p - v(0.0, 4.0)).length() - 5.0).abs() - 1.8).max(-sd_box(p, v(-3.5, 0.5), v(4.0, 3.5))), toon(col, v(0.0, 4.0), 6.0));
    c.paint(|p| sd_box(p, v(0.0, -2.5), v(1.8, 2.5)), flat(col));
    c.paint(|p| sd_circle(p, v(0.0, -8.5), 2.0), flat(col));
}

/// Paints `key`'s emblem; returns its number tag.
pub fn draw(key: &str, c: &mut Canvas) -> &'static str {
    match key {
        "first_blood" => {
            burst(c, v(0.0, 0.0), 13.0, RED, FIRE);
            tank(c, v(0.0, -0.5), 15.0, LIGHT, false);
            ""
        }
        "stage_1" => tagged(c, |c| flag(c, rgb(0.92, 0.30, 0.22)), "1"),
        "stage_10" => tagged(c, |c| flag(c, rgb(0.28, 0.56, 0.96)), "10"),
        "stage_20" => tagged(c, |c| flag(c, rgb(0.66, 0.36, 0.92)), "20"),
        "stage_30" => tagged(c, |c| flag(c, GOLD), "30"),
        "second_lap" => {
            let cyan = rgb(0.30, 0.82, 0.92);
            let ring = |p: Vec2| {
                let q = p - v(0.0, 1.0);
                let a = q.y.atan2(q.x).to_degrees();
                if (55.0..115.0).contains(&a) { 9.0 } else { (q.length() - 7.5).abs() - 1.9 }
            };
            c.paint(ring, toon(cyan, v(0.0, 1.0), 8.0));
            let head = [v(-1.0, 13.0), v(-1.0, 4.0), v(-7.5, 8.5)];
            c.paint(|p| sd_poly(p, &head), flat(cyan));
            "31"
        }
        "flawless" => {
            shield(c, v(0.0, 1.0), 9.0, STEEL);
            star5(c, v(0.0, 3.0), 4.5, GOLD);
            ""
        }
        "iron_wall" => {
            shield(c, v(0.0, 1.0), 9.0, rgb(0.44, 0.48, 0.56));
            for q in [v(-5.5, 7.5), v(5.5, 7.5), v(-5.5, 1.5), v(5.5, 1.5), v(0.0, -6.0)] {
                c.paint(|p| sd_circle(p, q, 1.1), flat(shade(STEEL, 1.3)));
            }
            c.paint(|p| sd_box(p, v(0.0, 3.0), v(1.0, 5.0)).min(sd_box(p, v(0.0, 3.5), v(4.0, 1.0))), flat(STEEL));
            "5"
        }
        "lone_wolf" => {
            c.paint(
                |p| sd_circle(p, v(-3.0, 4.0), 8.0).max(-sd_circle(p, v(1.5, 7.0), 7.0)),
                toon(rgb(1.0, 0.94, 0.66), v(-5.0, 4.0), 6.0),
            );
            for q in [v(6.0, 10.0), v(10.0, 4.0), v(-10.0, -6.0)] {
                c.paint(|p| sd_box(p, q, v(0.5, 0.5)), flat(WHITE));
            }
            tank(c, v(5.0, -5.0), 12.0, P1, true);
            ""
        }
        "duo_stage" => {
            c.paint(
                |p| {
                    sd_circle(p, v(-1.6, 9.5), 2.1)
                        .min(sd_circle(p, v(1.6, 9.5), 2.1))
                        .min(sd_poly(p, &[v(-3.6, 8.8), v(3.6, 8.8), v(0.0, 5.0)]))
                },
                flat(RED),
            );
            tank(c, v(-6.0, -3.0), 13.0, P1, true);
            tank(c, v(6.0, -3.0), 13.0, P2, true);
            ""
        }
        "duo_city" => {
            for (x, h) in [(-9.0, 6.0), (-3.0, 8.5), (3.0, 4.5), (9.0, 7.5)] {
                c.paint(|p| sd_box(p, v(x, -2.0 + h / 2.0), v(2.8, h / 2.0 + 4.0)), toon(rgb(0.30, 0.36, 0.56), v(x, 0.0), 3.0));
                for wy in [1.0, 4.5, 8.0].into_iter().filter(|&wy| wy < h - 2.0) {
                    c.paint(|p| sd_box(p, v(x, wy), v(0.6, 0.6)), flat(GOLD));
                }
            }
            tank(c, v(-5.5, -6.0), 11.0, P1, true);
            tank(c, v(5.5, -6.0), 11.0, P2, true);
            ""
        }
        "lan_party" => {
            for (x, y) in [(-5.5, 4.0), (5.5, -3.0)] {
                c.paint(|p| sd_box(p, v(x, y - 6.0), v(0.8, 2.0)).min(sd_box(p, v(x, y - 7.6), v(3.0, 0.7))), flat(STEEL));
                c.paint(|p| sd_rbox(p, v(x, y), v(5.4, 4.2), 0.8), flat(rgb(0.26, 0.28, 0.34)));
                c.paint(|p| sd_box(p, v(x, y), v(4.2, 3.0)), toon(rgb(0.30, 0.66, 1.0), v(x, y), 4.0));
            }
            c.paint(|p| sd_seg(p, v(-1.5, -1.0), v(1.0, 1.0), 0.7).min(sd_seg(p, v(1.0, 1.0), v(-0.5, 2.0), 0.7)), flat(GOLD));
            ""
        }
        "lifeline" => {
            let heart = |p: Vec2| {
                sd_circle(p, v(-3.4, 3.0), 4.6)
                    .min(sd_circle(p, v(3.4, 3.0), 4.6))
                    .min(sd_poly(p, &[v(-7.8, 1.2), v(7.8, 1.2), v(0.0, -8.5)]))
            };
            c.paint(heart, toon(RED, v(0.0, 1.0), 7.0));
            c.paint(|p| sd_box(p, v(0.0, 1.5), v(3.8, 1.2)).min(sd_box(p, v(0.0, 1.5), v(1.2, 3.8))), flat(WHITE));
            ""
        }
        // Bronze, silver, gold: one tank, a pair, a pile.
        "kills_100" => tagged(c, |c| tank(c, v(0.0, 1.0), 17.0, rgb(0.86, 0.58, 0.38), false), "100"),
        "kills_1000" => {
            tank(c, v(-4.5, 3.5), 13.0, rgb(0.66, 0.72, 0.84), false);
            tank(c, v(4.0, -2.5), 14.0, rgb(0.86, 0.90, 0.96), false);
            "1K"
        }
        "kills_5000" => {
            tank(c, v(-5.5, 4.5), 11.0, shade(GOLD, 0.85), false);
            tank(c, v(5.5, 4.5), 11.0, shade(GOLD, 0.85), false);
            tank(c, v(0.0, -3.5), 13.0, GOLD, false);
            "5K"
        }
        "can_opener" => tagged(
            c,
            |c| {
                tank(c, v(0.0, 1.0), 19.0, rgb(0.98, 0.45, 0.72), false);
                for x in [-2.6, 2.6] {
                    c.paint(|p| sd_box(p, v(x, 5.0), v(0.9, 1.6)), flat(rgb(0.70, 0.26, 0.48)));
                }
            },
            "100",
        ),
        "streak_10" => tagged(c, |c| flame(c, v(0.0, -1.0), 12.0, [RED, FIRE, rgb(1.0, 0.9, 0.4)]), "10"),
        "streak_25" => tagged(c, |c| flame(c, v(0.0, -1.0), 13.0, [FIRE, rgb(1.0, 0.86, 0.3), WHITE]), "25"),
        "streak_50" => tagged(c, |c| flame(c, v(0.0, -1.0), 14.0, [rgb(0.24, 0.40, 1.0), rgb(0.40, 0.84, 1.0), WHITE]), "50"),
        "score_20k" => tagged(c, |c| coins(c, v(-2.0, -8.0), 4), "20K"),
        "score_50k" => {
            coins(c, v(-5.5, -8.0), 5);
            coins(c, v(4.5, -8.0), 3);
            "50K"
        }
        "score_100k" => {
            coins(c, v(-6.5, -8.0), 4);
            coins(c, v(0.0, -8.0), 6);
            coins(c, v(6.5, -8.0), 3);
            star5(c, v(-7.0, 8.0), 3.0, WHITE);
            "100K"
        }
        "clean_sweep" => tagged(c, |c| grenade(c, v(-1.0, -1.0)), "4"),
        "six_pack" => {
            burst(c, v(0.0, 0.0), 13.0, FIRE, rgb(1.0, 0.86, 0.3));
            grenade(c, v(-1.0, -1.0));
            "6"
        }
        "overkill" => {
            c.paint(|p| sd_box(p, v(0.0, -9.5), v(11.0, 2.0)), toon(GRASS, v(0.0, -9.0), 3.0));
            for x in [-8.0, -3.0, 6.0] {
                c.paint(|p| sd_seg(p, v(x, -7.5), v(x + 1.0, -5.5), 0.5), flat(shade(GRASS, 1.3)));
            }
            grenade(c, v(0.0, 0.0));
            "0"
        }
        "fully_loaded" => {
            star5(c, v(-7.5, 6.5), 3.6, shade(GOLD, 0.9));
            star5(c, v(7.5, 6.5), 3.6, shade(GOLD, 0.9));
            star5(c, v(0.0, -1.0), 9.0, GOLD);
            "3"
        }
        _ => fun(key, c),
    }
}

/// Draws with `paint` and returns `tag`.
fn tagged(c: &mut Canvas, paint: impl FnOnce(&mut Canvas), tag: &'static str) -> &'static str {
    paint(c);
    tag
}

fn fun(key: &str, c: &mut Canvas) -> &'static str {
    match key {
        "friends_like_these" => {
            tank(c, v(0.0, -4.0), 14.0, P2, true);
            for (k, q) in [v(-7.0, 7.0), v(0.0, 10.0), v(7.0, 7.0)].into_iter().enumerate() {
                star5(c, q, if k == 1 { 3.4 } else { 2.8 }, rgb(1.0, 0.92, 0.4));
            }
            ""
        }
        "star_thief" => {
            let sack = rgb(0.64, 0.46, 0.26);
            c.paint(|p| sd_circle(p, v(-1.0, -3.5), 7.5), toon(sack, v(-1.0, -3.5), 7.0));
            c.paint(|p| sd_poly(p, &[v(-4.5, 3.0), v(2.5, 3.0), v(4.5, 7.5), v(-1.0, 5.0), v(-6.5, 7.5)]), toon(sack, v(-1.0, 5.0), 4.0));
            c.paint(|p| sd_box(p, v(-1.0, 3.3), v(3.6, 0.9)), flat(shade(sack, 0.55)));
            star5(c, v(5.5, 7.0), 4.8, GOLD);
            ""
        }
        "own_goal" => {
            c.paint(|p| sd_box(p, v(-3.0, -5.0), v(7.0, 5.0)), toon(STEEL, v(-3.0, -5.0), 6.0));
            star5(c, v(-3.0, -5.0), 3.5, GOLD);
            c.paint(|p| sd_circle(p, v(5.0, 5.5), 5.5), toon(WHITE, v(5.0, 5.5), 5.0));
            c.paint(|p| sd_poly(p, &star(v(5.0, 5.5), 2.2, 2.0, 5)), flat(DARK));
            for y in [9.0, 5.0, 1.0] {
                c.paint(|p| sd_seg(p, v(-9.0, y + 1.0), v(-3.5, y + 1.0), 0.5), flat(WHITE));
            }
            ""
        }
        "borrowed_time" => {
            let glass = rgb(0.70, 0.86, 1.0);
            c.paint(
                |p| sd_poly(p, &[v(-6.0, 8.0), v(6.0, 8.0), v(0.6, 0.0), v(6.0, -8.0), v(-6.0, -8.0), v(-0.6, 0.0)]),
                toon(glass, v(0.0, 0.0), 6.0),
            );
            c.paint(|p| sd_poly(p, &[v(-4.0, 5.0), v(4.0, 5.0), v(0.0, 1.0)]), flat(GOLD));
            c.paint(|p| sd_poly(p, &[v(-5.2, -7.0), v(5.2, -7.0), v(0.0, -3.5)]).min(sd_box(p, v(0.0, -2.0), v(0.35, 2.0))), flat(GOLD));
            for y in [9.5, -9.5] {
                c.paint(|p| sd_rbox(p, v(0.0, y), v(8.0, 1.6), 0.6), toon(WOOD, v(0.0, y), 2.0));
            }
            ""
        }
        "loan_shark" => {
            c.paint(
                |p| sd_poly(p, &[v(-8.0, -4.0), v(-1.0, 3.0), v(1.0, 10.0), v(6.0, -4.0)]),
                toon(rgb(0.56, 0.62, 0.70), v(0.0, 2.0), 6.0),
            );
            c.paint(|p| (p.y + 5.0 + (p.x * 0.9).sin()).max(sd_box(p, v(0.0, -8.0), v(11.0, 6.0))), toon(WATER, v(0.0, -7.0), 4.0));
            c.paint(|p| sd_circle(p, v(-6.0, -6.0), 4.0), toon(GOLD, v(-6.0, -6.0), 3.0));
            c.paint(|p| sd_circle(p, v(-6.0, -6.0), 2.4).abs() - 0.45, flat(shade(GOLD, 0.65)));
            "5"
        }
        "conscientious_objector" => {
            c.paint(|p| sd_box(p, v(-6.0, -1.0), v(0.9, 11.0)), toon(WOOD, v(-6.0, 0.0), 1.0));
            let cloth = |p: Vec2| sd_box(p, v(2.5, 6.0), v(7.5, 5.0)).max(-(p.y - 1.5 - (p.x * 0.8).sin()));
            c.paint(|p| cloth(p - v(0.0, (p.x * 0.6).sin() * 0.8)), toon(WHITE, v(2.0, 6.0), 5.0));
            ""
        }
        "backseat_driver" => {
            c.paint(
                |p| sd_poly(p, &[v(-7.5, 4.0), v(7.5, 4.0), v(5.0, -11.0), v(-5.0, -11.0)]),
                |p| {
                    if (p.x * 0.42 + 9.0) as i32 % 2 == 0 { RED } else { WHITE }
                },
            );
            for q in [v(-5.0, 5.5), v(-1.5, 8.0), v(2.5, 7.5), v(5.5, 5.0), v(0.5, 4.8), v(-3.0, 9.8)] {
                c.paint(|p| sd_circle(p, q, 2.6), toon(rgb(1.0, 0.95, 0.74), q, 2.0));
            }
            ""
        }
        "mutual_destruction" => {
            tank(c, v(0.0, 7.0), 10.0, LIGHT, false);
            tank(c, v(0.0, -7.0), 10.0, P1, true);
            burst(c, v(0.0, 0.0), 6.5, FIRE, rgb(1.0, 0.94, 0.6));
            ""
        }
        "hanging_by_a_thread" => {
            c.paint(|p| sd_box(p, v(0.0, 8.0), v(0.6, 4.5)), flat(rgb(0.92, 0.92, 0.92)));
            c.paint(|p| sd_box(p, v(0.0, -1.0), v(5.5, 5.0)), toon(STEEL, v(0.0, -1.0), 5.0));
            star5(c, v(0.0, -1.0), 3.0, GOLD);
            for (x, y) in [(-9.0, -9.0), (-5.5, -10.0), (7.0, -9.5), (9.5, -7.0)] {
                c.paint(|p| sd_box(p, v(x, y), v(1.4, 0.9)), flat(rgb(0.76, 0.38, 0.24)));
            }
            ""
        }
        "ff_enthusiast" => {
            let bolt = [v(3.0, 11.0), v(-6.5, -1.0), v(-0.5, -1.0), v(-3.0, -11.0), v(7.0, 2.5), v(1.0, 2.5), v(5.0, 11.0)];
            c.paint(|p| sd_poly(p, &bolt), toon(rgb(1.0, 0.90, 0.30), v(0.0, 0.0), 8.0));
            "10"
        }
        "bathroom_break" => {
            c.paint(|p| sd_box(p, v(5.5, -3.0), v(2.0, 7.0)), |p| if (p.y + 20.0) as i32 % 4 == 0 { shade(WHITE, 0.78) } else { WHITE });
            c.paint(|p| sd_rbox(p, v(-2.0, 2.0), v(6.5, 7.5), 1.5), toon(WHITE, v(-2.0, 2.0), 6.0));
            c.paint(|p| sd_circle(p, v(-2.0, 8.0), 2.0), flat(rgb(0.62, 0.50, 0.38)));
            "10"
        }
        "bullet_ballet" => {
            c.paint(|p| sd_seg(p, v(-10.0, -10.0), v(-3.5, -3.5), 2.2), toon(GOLD, v(-7.0, -7.0), 3.0));
            c.paint(|p| sd_seg(p, v(10.0, 10.0), v(3.5, 3.5), 2.2), toon(RED, v(7.0, 7.0), 3.0));
            burst(c, v(0.0, 0.0), 7.0, WHITE, rgb(1.0, 0.92, 0.5));
            "100"
        }
        "look_both_ways" => {
            // A zebra crossing under a double-headed arrow.
            c.paint(|p| sd_box(p, v(0.0, -5.0), v(10.5, 5.0)), flat(rgb(0.24, 0.24, 0.27)));
            for x in [-7.5, -2.5, 2.5, 7.5] {
                c.paint(|p| sd_box(p, v(x, -5.0), v(1.5, 4.0)), toon(WHITE, v(x, -5.0), 4.0));
            }
            let arrow = |p: Vec2| {
                let heads = [-1.0, 1.0].map(|s| sd_poly(p, &[v(s * 10.5, 6.5), v(s * 5.0, 11.0), v(s * 5.0, 2.0)]));
                sd_box(p, v(0.0, 6.5), v(6.0, 1.4)).min(heads[0]).min(heads[1])
            };
            c.paint(arrow, toon(GOLD, v(0.0, 6.5), 5.0));
            ""
        }
        _ => badge_city::draw(key, c),
    }
}
