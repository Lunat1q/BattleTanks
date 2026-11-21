//! Achievement badges: 32x32 pixel art, one per entry. A bevelled plate in the group's colour with a gold rim, the
//! entry's emblem (`badge_art`) drawn crisp with a dark outline and a drop shadow, and a small number tag where the
//! entry has one (stage 10, 1K kills...). Each badge also has a locked copy (cold grey, dim). Shown with nearest
//! sampling so they stay chunky on a TV; the Play Console icons are the same pixels scaled up.

use super::badge_art;
use super::canvas::*;
use crate::achieve::LIST;
use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::Image;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

pub const PX: usize = 32;
type Px = Vec<[u8; 4]>;

const OUTLINE: [u8; 4] = [12, 11, 16, 255];
const SHADOW: [u8; 4] = [0, 0, 0, 120];

/// Plate colour per group (list order of `achieve::list::TABS`).
const PLATES: [C; 6] = [
    rgb(0.21, 0.33, 0.17), // classic: olive
    rgb(0.12, 0.31, 0.33), // solo and duo: teal
    rgb(0.17, 0.23, 0.40), // City Run: slate
    rgb(0.30, 0.19, 0.40), // City Run more: violet
    rgb(0.42, 0.15, 0.15), // combat: crimson
    rgb(0.42, 0.28, 0.10), // fun: amber
];

/// Straight-alpha RGBA8 pixels of entry `i`'s badge, row 0 at the top (`locked`: the grey copy).
pub fn pixels(i: usize, locked: bool) -> Vec<u8> {
    let a = &LIST[i];
    let group = crate::achieve::list::TABS.iter().position(|t| t.0 == a.group).unwrap_or(0);
    let mut c = Canvas::new(PX as f32, PX as f32, 1.0);
    let tag = badge_art::draw(a.key, &mut c);
    let mut px = compose(PLATES[group], &c);
    if !tag.is_empty() {
        label(&mut px, tag);
    }
    if locked {
        dim(&mut px);
    }
    px.into_iter().flatten().collect()
}

/// The badge of a hidden entry nobody has found yet: a dark plate with a question mark.
pub fn hidden_pixels() -> Vec<u8> {
    let mut c = Canvas::new(PX as f32, PX as f32, 1.0);
    badge_art::question(&mut c);
    let mut px = compose(rgb(0.16, 0.16, 0.20), &c);
    dim(&mut px);
    px.into_iter().flatten().collect()
}

pub fn badge(i: usize, locked: bool) -> Image {
    image(pixels(i, locked), PX)
}

pub fn hidden() -> Image {
    image(hidden_pixels(), PX)
}

/// The points diamond next to each row's points.
pub fn gem() -> Image {
    const ROWS: [&str; 9] =
        ["....k....", "...kwk...", "..kwyyk..", ".kwyyyok.", "kwyyyyook", ".kyyyook.", "..kyook..", "...kok...", "....k...."];
    let px = ROWS
        .iter()
        .flat_map(|r| r.bytes())
        .flat_map(|b| match b {
            b'k' => OUTLINE,
            b'w' => [255, 248, 210, 255],
            b'y' => [255, 204, 77, 255],
            b'o' => [196, 128, 36, 255],
            _ => [0; 4],
        })
        .collect();
    image(px, 9)
}

fn image(data: Vec<u8>, size: usize) -> Image {
    let mut img = Image::new(
        Extent3d { width: size as u32, height: size as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    img.sampler = ImageSampler::nearest();
    img
}

fn to8(c: C) -> [u8; 4] {
    c.map(|v| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8)
}

/// Plate: rounded square, dark outline, a gold rim lit from the top left, a dark groove, then the face in three
/// bands (light top, dark bottom) with a checker dither where the bands meet.
fn plate(base: C) -> Px {
    let n = PX as i32;
    let gold = rgb(1.0, 0.80, 0.30);
    (0..n * n)
        .map(|i| {
            let (x, y) = (i % n, i / n);
            // Distance in from the nearest edge, with the corners cut round.
            let (ex, ey) = (x.min(n - 1 - x), y.min(n - 1 - y));
            let corner = if ex < 3 && ey < 3 { ((3 - ex) * (3 - ex) + (3 - ey) * (3 - ey)) as f32 } else { 0.0 };
            let edge = ex.min(ey) - (corner > 5.0) as i32 - (corner > 10.0) as i32;
            let lit = x + y < n; // top-left half
            match edge {
                ..0 => [0; 4],
                0 => OUTLINE,
                1 => to8(shade(gold, if lit { 1.15 } else { 0.62 })),
                2 => to8(shade(base, 0.45)),
                _ => {
                    let band = (y as f32 / n as f32 * 3.0).min(2.0);
                    let b = band as i32;
                    let k = [1.22, 1.0, 0.78][b as usize];
                    // Dither one row into the next band.
                    let k = if (y * 3) % n < 3 && b > 0 && (x + y) % 2 == 0 { [1.22, 1.0, 0.78][b as usize - 1] } else { k };
                    let shine = edge == 3 && lit;
                    to8(shade(base, if shine { k * 1.25 } else { k }))
                }
            }
        })
        .collect()
}

/// Emblem `c` made crisp (no half-covered pixels), outlined, shadowed and laid on the plate.
fn compose(base: C, c: &Canvas) -> Px {
    let mut out = plate(base);
    let raw = c.rgba8();
    let glyph: Px = raw.chunks(4).map(|p| if p[3] >= 110 { [p[0], p[1], p[2], 255] } else { [0; 4] }).collect();
    let n = PX as i32;
    let solid = |x: i32, y: i32| x >= 0 && y >= 0 && x < n && y < n && glyph[(y * n + x) as usize][3] > 0;
    let inked = |x: i32, y: i32| solid(x, y) || [(1, 0), (-1, 0), (0, 1), (0, -1)].iter().any(|(dx, dy)| solid(x + dx, y + dy));
    for y in 0..n {
        for x in 0..n {
            let i = (y * n + x) as usize;
            // The rim stays whole: emblems end inside it.
            if out[i][3] == 0 || x < 3 || y < 3 || x >= n - 3 || y >= n - 3 {
                continue;
            }
            if solid(x, y) {
                out[i] = glyph[i];
            } else if inked(x, y) {
                out[i] = OUTLINE;
            } else if inked(x - 1, y - 1) {
                out[i] = blend(out[i], SHADOW);
            }
        }
    }
    out
}

fn blend(d: [u8; 4], s: [u8; 4]) -> [u8; 4] {
    let a = s[3] as u32;
    let m = |k: usize| ((s[k] as u32 * a + d[k] as u32 * (255 - a)) / 255) as u8;
    [m(0), m(1), m(2), d[3]]
}

/// 3x5 digits (and K) for the number tags.
fn glyph3(ch: char) -> [u8; 5] {
    match ch {
        '0' => [7, 5, 5, 5, 7],
        '1' => [2, 6, 2, 2, 7],
        '2' => [7, 1, 7, 4, 7],
        '3' => [7, 1, 3, 1, 7],
        '4' => [5, 5, 7, 1, 1],
        '5' => [7, 4, 7, 1, 7],
        '6' => [7, 4, 7, 5, 7],
        '7' => [7, 1, 1, 2, 2],
        '8' => [7, 5, 7, 5, 7],
        '9' => [7, 5, 7, 1, 7],
        'K' => [5, 5, 6, 5, 5],
        _ => [0; 5],
    }
}

/// White number tag in the bottom-right corner, outlined so it reads over any emblem.
fn label(px: &mut Px, s: &str) {
    let n = PX as i32;
    let w = s.chars().count() as i32 * 4 - 1;
    let (x0, y0) = (n - 5 - w, n - 10);
    let on = |x: i32, y: i32| {
        let (cx, cy) = (x - x0, y - y0);
        if !(0..w).contains(&cx) || !(0..5).contains(&cy) || cx % 4 == 3 {
            return false;
        }
        let ch = s.chars().nth((cx / 4) as usize).unwrap_or(' ');
        glyph3(ch)[cy as usize] >> (2 - cx % 4) & 1 == 1
    };
    for y in y0 - 1..y0 + 6 {
        for x in x0 - 1..x0 + w + 1 {
            let i = (y * n + x) as usize;
            if on(x, y) {
                px[i] = if y - y0 < 2 { [255, 255, 255, 255] } else { [255, 236, 190, 255] };
            } else if (-1..=1).any(|dy| (-1..=1).any(|dx| on(x + dx, y + dy))) {
                px[i] = OUTLINE;
            }
        }
    }
}

/// Locked look: cold grey at half brightness.
fn dim(px: &mut Px) {
    for p in px.iter_mut() {
        let l = (p[0] as f32 * 0.3 + p[1] as f32 * 0.59 + p[2] as f32 * 0.11) * 0.5;
        *p = [(l + 14.0) as u8, (l + 16.0) as u8, (l + 24.0) as u8, p[3]];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_badge_has_an_emblem_and_differs_from_the_rest() {
        let all: Vec<Vec<u8>> = (0..LIST.len()).map(|i| pixels(i, false)).collect();
        let blank = {
            let mut c = Canvas::new(PX as f32, PX as f32, 1.0);
            c.fill(|_| [0.0; 4]);
            compose(PLATES[0], &c).into_iter().flatten().collect::<Vec<u8>>()
        };
        for (i, px) in all.iter().enumerate() {
            assert_eq!(px.len(), PX * PX * 4);
            let changed = px.chunks(4).zip(blank.chunks(4)).filter(|(a, b)| a != b).count();
            assert!(changed > 60, "{} has almost no emblem ({changed} px)", LIST[i].key);
            for j in 0..i {
                assert_ne!(all[j], *px, "{} looks like {}", LIST[i].key, LIST[j].key);
            }
        }
        assert_ne!(pixels(0, true), all[0]);
    }

    /// Contact sheet of every badge (unlocked row, locked below), 6x, to look at: `BT_BADGE_SHEET=<png> cargo test
    /// badge_sheet -- --ignored`.
    #[test]
    #[ignore = "writes a preview PNG to BT_BADGE_SHEET"]
    fn badge_sheet() {
        let Ok(path) = std::env::var("BT_BADGE_SHEET") else { return };
        let (cols, scale, gap) = (11, 6, 4);
        let cell = PX * scale + gap;
        let rows = LIST.len().div_ceil(cols) * 2 + 1;
        let (w, h) = (cols * cell + gap, rows * cell + gap);
        let mut out = vec![30u8; w * h * 4];
        let mut put = |px: &[u8], col: usize, row: usize| {
            for y in 0..PX * scale {
                for x in 0..PX * scale {
                    let s = ((y / scale) * PX + x / scale) * 4;
                    let d = ((gap + row * cell + y) * w + gap + col * cell + x) * 4;
                    let a = px[s + 3] as u32;
                    for k in 0..3 {
                        out[d + k] = ((px[s + k] as u32 * a + out[d + k] as u32 * (255 - a)) / 255) as u8;
                    }
                    out[d + 3] = 255;
                }
            }
        };
        let half = LIST.len().div_ceil(cols);
        for i in 0..LIST.len() {
            put(&pixels(i, false), i % cols, i / cols);
            put(&pixels(i, true), i % cols, half + i / cols);
        }
        put(&hidden_pixels(), 0, half * 2);
        let img = Image::new(
            Extent3d { width: w as u32, height: h as u32, depth_or_array_layers: 1 },
            TextureDimension::D2,
            out,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::all(),
        );
        img.try_into_dynamic().unwrap().save(path).unwrap();
    }
}
