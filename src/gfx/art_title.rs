//! Title screen art: the "BATTLE TANKS" logo built from the game's own terrain ("BATTLE" in brick, "TANKS" in steel)
//! and the pixel-art frame of the player cards.

use super::art_promo::GLYPHS;
use super::art_world::{brick_at, steel_at};
use super::canvas::*;
use bevy::image::ImageSampler;
use bevy::math::Vec2;
use bevy::prelude::Image;

/// One glyph pixel is one 8-unit terrain cell: a steel plate, or two bricks by four courses.
const CELL: f32 = 8.0;
/// Cells between the two words.
const GAP: usize = 3;
/// Drop shadow offset, units.
const SHADOW: f32 = 3.0;

fn lit(text: &str, col: usize, row: usize) -> bool {
    let Some(ch) = text.chars().nth(col / 6) else { return false };
    let Some((_, rows)) = GLYPHS.iter().find(|g| g.0 == ch) else { return false };
    col % 6 < 5 && rows[row].as_bytes()[col % 6] == b'#'
}

/// Material of glyph cell `(col, row)` from the top-left: `Some(true)` brick, `Some(false)` steel, `None` empty.
fn cell(col: usize, row: usize) -> Option<bool> {
    let battle = 6 * 6 - 1;
    if row >= 7 {
        None
    } else if col < battle {
        lit("BATTLE", col, row).then_some(true)
    } else {
        col.checked_sub(battle + GAP).filter(|&c| lit("TANKS", c, row)).map(|_| false)
    }
}

/// The logo at `k` px per unit, transparent around the letters.
pub fn logo(k: f32) -> Image {
    let cols = 6 * 6 - 1 + GAP + 5 * 6 - 1;
    let (w, h) = (cols as f32 * CELL + SHADOW, 7.0 * CELL + SHADOW);
    let mut c = Canvas::new(w, h, k);
    let at = |x: f32, y: f32| (x >= 0.0 && y >= 0.0).then(|| cell((x / CELL) as usize, (y / CELL) as usize)).flatten();
    c.fill(|p| {
        // From the top-left, y down, for the glyph grid; the patterns want y up and positive.
        let (x, y) = (p.x + w / 2.0, h / 2.0 - p.y);
        match at(x, y) {
            Some(true) => brick_at(Vec2::new(x, h - y)),
            Some(false) => steel_at(Vec2::new(x, h - y)),
            None if at(x - SHADOW, y - SHADOW).is_some() => [0.0, 0.0, 0.0, 0.55],
            None => [0.0; 4],
        }
    });
    c.into_image()
}

/// Player card frame, `W x H` chunky pixels, greyscale (the card tints it with the player's colour): notched
/// corners, a bright outline, a bevel (light top-left, dark bottom-right) and a dark, slightly see-through fill.
pub const FRAME: (usize, usize) = (30, 22);

pub fn card_frame() -> Image {
    let (w, h) = FRAME;
    let mut c = Canvas::new(w as f32, h as f32, 1.0);
    c.fill(|p| {
        let (x, y) = ((p.x + w as f32 / 2.0) as usize, (h as f32 / 2.0 - p.y) as usize);
        let edge = x.min(w - 1 - x).min(y).min(h - 1 - y);
        let corner = x.min(w - 1 - x) + y.min(h - 1 - y);
        match (edge, corner) {
            // Two-pixel notch at each corner, the outline stepping round it.
            (_, 0 | 1) => [0.0; 4],
            (0, _) | (_, 2) => gray(1.0),
            (1, _) if x == 1 || y == 1 => gray(0.62),
            (1, _) => gray(0.30),
            _ => [0.12, 0.12, 0.12, 0.92],
        }
    });
    let mut img = c.into_image();
    img.sampler = ImageSampler::nearest();
    img
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logo_words_use_their_material() {
        assert_eq!(cell(0, 0), Some(true), "B's top-left is brick");
        assert_eq!(cell(5, 0), None, "space between letters");
        assert_eq!(cell(35 + GAP, 0), Some(false), "T of TANKS is steel");
        assert_eq!(cell(36, 3), None, "the gap between the words");
        assert_eq!(cell(0, 7), None);
    }
}
