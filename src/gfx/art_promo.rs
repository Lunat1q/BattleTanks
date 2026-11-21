//! Store and launcher art painted from the game's own icon and tanks: the title banner (Android TV / Google Play,
//! via `paint_banner` in the wrapper crate) and Steam library art for the non-Steam shortcut:
//! `cargo test write_steam_art -- --ignored` writes `docs/steam/*.png` (commit them).

use super::canvas::*;
use super::icon::paint as paint_icon;
use bevy::math::Vec2;

/// 5x7 pixel glyphs for "BATTLE TANKS"; `#` = lit.
pub(super) const GLYPHS: [(char, [&str; 7]); 8] = [
    ('A', [".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"]),
    ('B', ["####.", "#...#", "#...#", "####.", "#...#", "#...#", "####."]),
    ('E', ["#####", "#....", "#....", "####.", "#....", "#....", "#####"]),
    ('K', ["#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#"]),
    ('L', ["#....", "#....", "#....", "#....", "#....", "#....", "#####"]),
    ('N', ["#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#", "#...#"]),
    ('S', [".####", "#....", "#....", ".###.", "....#", "....#", "####."]),
    ('T', ["#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#.."]),
];
const TITLE_GOLD: C = [0.98, 0.80, 0.34, 1.0];

fn backdrop(p: Vec2, height: f32) -> C {
    mix(rgb(0.05, 0.07, 0.13), rgb(0.14, 0.19, 0.34), 0.5 + p.y / height)
}

#[cfg(test)]
/// Width of `text` in glyph cells (5 per letter, 1 between).
fn cells(text: &str) -> f32 {
    (text.chars().count() * 6 - 1) as f32
}

/// `text` in glyph pixels `cell` units square, its top-left at `(x, y)` units from the canvas top-left (`k` px/unit).
/// Each lit pixel is a blit of one small square, so big canvases stay quick.
fn word(c: &mut Canvas, k: f32, text: &str, x: f32, y: f32, cell: f32, color: C) {
    let mut dot = Canvas::new(cell, cell, k);
    dot.fill(|_| color);
    for (n, ch) in text.chars().enumerate() {
        let Some((_, rows)) = GLYPHS.iter().find(|g| g.0 == ch) else { continue };
        for (r, row) in rows.iter().enumerate() {
            for (col, _) in row.chars().enumerate().filter(|(_, b)| *b == '#') {
                let (px, py) = (x + (n * 6 + col) as f32 * cell, y + r as f32 * cell);
                c.blit(&dot, (px * k).round() as usize, (py * k).round() as usize);
            }
        }
    }
}

/// Banner `w x h` units at `k` px per unit: the icon tile beside "BATTLE / TANKS", centred on a 320 x 180 layout.
/// Android launcher banner = (320, 180, 2); Play TV banner = (320, 180, 4); Play feature graphic = (409.6, 200, 2.5);
/// Steam wide capsule = (368, 172, 2.5).
pub fn banner(w: f32, h: f32, k: f32) -> Canvas {
    let (dx, dy) = ((w - 320.0) / 2.0, (h - 180.0) / 2.0);
    let mut c = Canvas::new(w, h, k);
    c.fill(|p| backdrop(p, h));
    c.blit(&paint_icon((140.0 * k) as u32), ((16.0 + dx) * k) as usize, ((20.0 + dy) * k) as usize);
    for (line, text) in ["BATTLE", "TANKS"].iter().enumerate() {
        word(&mut c, k, text, 170.0 + dx, 50.0 + dy + line as f32 * 44.0, 4.0, TITLE_GOLD);
    }
    c
}

#[cfg(test)]
/// Steam portrait capsule (600 x 900): icon on top, the title under it.
pub fn capsule_tall() -> Canvas {
    let (w, h, k) = (300.0, 450.0, 2.0);
    let mut c = Canvas::new(w, h, k);
    c.fill(|p| backdrop(p, h));
    c.blit(&paint_icon((210.0 * k) as u32), ((w - 210.0) / 2.0 * k) as usize, (50.0 * k) as usize);
    let cell = 7.0;
    for (line, text) in ["BATTLE", "TANKS"].iter().enumerate() {
        word(&mut c, k, text, (w - cells(text) * cell) / 2.0, 295.0 + line as f32 * 66.0, cell, TITLE_GOLD);
    }
    c
}

#[cfg(test)]
/// Steam logo (1280 x 200, transparent): the title in one line with a dark outline, laid over the hero.
pub fn logo() -> Canvas {
    let (w, h, k, cell) = (1280.0, 200.0, 1.0, 16.0);
    let mut c = Canvas::new(w, h, k);
    let (x, y) = ((w - cells("BATTLE TANKS") * cell) / 2.0, (h - 7.0 * cell) / 2.0);
    let edge = 5.0;
    let mut shadow = Canvas::new(cell + 2.0 * edge, cell + 2.0 * edge, k);
    shadow.fill(|_| [0.03, 0.04, 0.08, 0.92]);
    // Outline first (each lit pixel's square, grown), then the gold on top.
    for (n, ch) in "BATTLE TANKS".chars().enumerate() {
        let Some((_, rows)) = GLYPHS.iter().find(|g| g.0 == ch) else { continue };
        for (r, row) in rows.iter().enumerate() {
            for (col, _) in row.chars().enumerate().filter(|(_, b)| *b == '#') {
                let (px, py) = (x + (n * 6 + col) as f32 * cell - edge, y + r as f32 * cell - edge);
                c.blit(&shadow, px as usize, py as usize);
            }
        }
    }
    word(&mut c, k, "BATTLE TANKS", x, y, cell, TITLE_GOLD);
    c
}

#[cfg(test)]
/// Steam hero (3840 x 1240, no text: Steam lays the logo over the empty left side): a dark field with a faint grid,
/// a brick wall, the two players' tanks in front of it and enemies coming down on the right.
pub fn hero() -> Canvas {
    use crate::rules::EnemyKind;
    let (w, h, k) = (1920.0, 620.0, 2.0);
    let mut c = Canvas::new(w, h, k);
    c.fill(|p| {
        let grid = ((p.x / 32.0).fract().abs() < 0.04 || (p.y / 32.0).fract().abs() < 0.04) as u8 as f32;
        // Brick band: 24-unit courses, bricks 48 wide, every other course offset, mortar between.
        let band = p.y < 30.0 && p.y > -66.0;
        let course = ((p.y + 66.0) / 24.0).floor();
        let bx = (p.x + if course as i32 % 2 == 0 { 0.0 } else { 24.0 }).rem_euclid(48.0);
        let by = (p.y + 66.0).rem_euclid(24.0);
        if band {
            let mortar = bx < 3.0 || by < 3.0;
            let tone = 0.85 + 0.15 * hash(((p.x + 2000.0) / 48.0) as i32, course as i32, 7);
            if mortar { rgb(0.30, 0.27, 0.25) } else { shade(rgb(0.70, 0.33, 0.20), tone) }
        } else {
            mix(backdrop(p, h), rgb(0.25, 0.32, 0.55), grid * 0.18)
        }
    });
    // Tanks are 16-unit models: paint at `size / 16` px per unit, then place by their top-left in units.
    let mut place = |canvas: Canvas, x: f32, y: f32| c.blit(&canvas, (x * k) as usize, (y * k) as usize);
    for (kind, x, tint) in [
        (EnemyKind::Power, 1180.0, rgb(0.95, 0.55, 0.45)),
        (EnemyKind::Armor, 1440.0, rgb(0.75, 0.80, 0.95)),
        (EnemyKind::Fast, 1700.0, rgb(0.95, 0.55, 0.45)),
    ] {
        let size = 120.0;
        let mut t = super::art_tanks::enemy_art(kind, size / 16.0 * k, 16.0).flipped();
        t.tint(tint);
        place(t, x, 40.0);
    }
    for (x, tint) in [(1250.0, rgb(1.0, 0.80, 0.32)), (1560.0, rgb(0.42, 0.92, 0.52))] {
        let size = 190.0;
        let mut t = super::art_tanks::hero(size / 16.0 * k, 16.0);
        t.tint(tint);
        place(t, x, h - size - 90.0);
    }
    // Shots in flight between them, with a soft glow.
    for (x, y) in [(1345.0, 240.0), (1655.0, 170.0), (1500.0, 205.0)] {
        let at = Vec2::new(x - w / 2.0, h / 2.0 - y);
        c.paint(|p| sd_circle(p, at, 16.0), |p| [1.0, 0.85, 0.5, 0.35 * (1.0 - p.distance(at) / 16.0)]);
        c.paint(|p| sd_rbox(p, at, Vec2::new(3.0, 7.0), 3.0), |_| rgb(1.0, 0.95, 0.8));
    }
    c
}

/// PNG writer for the art tests (`opaque` drops alpha: Play wants 24-bit listing graphics).
#[cfg(test)]
pub fn save_png(c: &Canvas, path: &str, opaque: bool) {
    use bevy::prelude::Image;
    let (w, h) = c.size();
    let img = Image::new(
        bevy::render::render_resource::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
        bevy::render::render_resource::TextureDimension::D2,
        c.rgba8(),
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::all(),
    );
    std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();
    let img = img.try_into_dynamic().unwrap();
    if opaque { img.to_rgb8().save(path) } else { img.save(path) }.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn art_sizes() {
        assert_eq!(banner(320.0, 180.0, 2.0).size(), (640, 360));
        assert_eq!(banner(368.0, 172.0, 2.5).size(), (920, 430));
        assert_eq!(capsule_tall().size(), (600, 900));
        assert_eq!(logo().size(), (1280, 200));
    }

    #[test]
    fn flip_turns_a_canvas_upside_down() {
        let mut c = Canvas::new(1.0, 2.0, 1.0);
        c.paint(|p| -p.y, |_| rgb(1.0, 0.0, 0.0)); // top half only
        let (top, flipped) = (c.rgba8(), c.flipped().rgba8());
        assert_eq!((top[3], top[7]), (255, 0));
        assert_eq!((flipped[3], flipped[7]), (0, 255));
    }

    #[test]
    #[ignore = "writes docs/steam/*.png; run when the art changes"]
    fn write_steam_art() {
        save_png(&capsule_tall(), "docs/steam/capsule-600x900.png", true);
        save_png(&banner(368.0, 172.0, 2.5), "docs/steam/capsule-920x430.png", true);
        save_png(&hero(), "docs/steam/hero-3840x1240.png", true);
        save_png(&logo(), "docs/steam/logo-1280x200.png", false);
        save_png(&paint_icon(256), "docs/steam/icon-256.png", false);
    }
}
