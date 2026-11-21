//! Block-built text (the title logo, screen headings): every glyph pixel is one 8-unit terrain cell of brick, steel,
//! water, ice, trees or gold, painted with the field's own patterns; and the pixel-art frame of the player cards.

use super::art_world::{brick_at, gold_at, ice_at, steel_at, trees_at, water_at};
use super::canvas::*;
use super::font::glyph;
use bevy::image::ImageSampler;
use bevy::math::Vec2;
use bevy::prelude::Image;

/// One glyph pixel is one 8-unit terrain cell: a steel plate, or two bricks by four courses.
const CELL: f32 = 8.0;
/// Drop shadow offset, units.
const SHADOW: f32 = 3.0;

/// What a block word is built from.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Block {
    Brick,
    Steel,
    Water,
    Ice,
    Trees,
    Gold,
}

impl Block {
    fn at(self, q: Vec2) -> C {
        match self {
            Block::Brick => brick_at(q),
            Block::Steel => steel_at(q),
            // Water and trees are dark field textures, lifted so letters of them read on a black screen.
            Block::Water => shade(water_at(q, 0.0), 1.7),
            Block::Ice => ice_at(q),
            Block::Trees => shade(trees_at(q), 1.5),
            Block::Gold => gold_at(q),
        }
    }
}

/// Glyph cells of `words` (a space between them), 7 rows from the top-left, row-major: `(columns, cells)`. A letter
/// takes 5 columns plus one apart, a space 2 more, so words sit 3 columns apart.
fn cells(words: &[(&str, Block)]) -> (usize, Vec<Option<Block>>) {
    let mut lit = Vec::new();
    let mut x = 0;
    for (i, &(word, m)) in words.iter().enumerate() {
        for ch in (if i > 0 { " " } else { "" }).chars().chain(word.chars()) {
            let Some(rows) = glyph(ch.to_ascii_uppercase()) else {
                x += 2;
                continue;
            };
            for (r, row) in rows.iter().enumerate() {
                lit.extend(row.bytes().enumerate().filter(|&(_, b)| b == b'#').map(|(c, _)| (x + c, r, m)));
            }
            x += 6;
        }
    }
    let cols = x.saturating_sub(1).max(1);
    let mut grid = vec![None; cols * 7];
    for (c, r, m) in lit {
        grid[r * cols + c] = Some(m);
    }
    (cols, grid)
}

/// Size of `block_text(words, _)` in units.
pub fn block_size(words: &[(&str, Block)]) -> Vec2 {
    Vec2::new(cells(words).0 as f32 * CELL + SHADOW, 7.0 * CELL + SHADOW)
}

/// `words` built from their blocks at `k` px per unit, with a drop shadow, transparent around the letters.
pub fn block_text(words: &[(&str, Block)], k: f32) -> Image {
    let (cols, grid) = cells(words);
    let Vec2 { x: w, y: h } = block_size(words);
    let mut c = Canvas::new(w, h, k);
    let at = |x: f32, y: f32| {
        let (col, row) = ((x / CELL) as usize, (y / CELL) as usize);
        (x >= 0.0 && y >= 0.0 && col < cols && row < 7).then(|| grid[row * cols + col]).flatten()
    };
    c.fill(|p| {
        // From the top-left, y down, for the glyph grid; the patterns want y up and positive.
        let (x, y) = (p.x + w / 2.0, h / 2.0 - p.y);
        match at(x, y) {
            Some(m) => m.at(Vec2::new(x, h - y)),
            None if at(x - SHADOW, y - SHADOW).is_some() => [0.0, 0.0, 0.0, 0.55],
            None => [0.0; 4],
        }
    });
    c.into_image()
}

/// The title logo: "BATTLE" in brick, "TANKS" in steel.
pub const LOGO: [(&str, Block); 2] = [("BATTLE", Block::Brick), ("TANKS", Block::Steel)];

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
    fn block_words_use_their_material() {
        let (cols, g) = cells(&LOGO);
        assert_eq!(cols, 67, "BATTLE, 3 columns apart, TANKS");
        let at = |c: usize, r: usize| g[r * cols + c];
        assert_eq!(at(0, 0), Some(Block::Brick), "B's top-left is brick");
        assert_eq!(at(5, 0), None, "space between letters");
        assert_eq!(at(38, 0), Some(Block::Steel), "T of TANKS is steel");
        assert_eq!(at(36, 3), None, "the gap between the words");
        let (cols, g) = cells(&[("lan co-op", Block::Water)]);
        assert_eq!(cols, 9 * 6 - 1 - 4, "lower case is upper case, a space is 2 columns");
        assert!(g.iter().flatten().all(|&m| m == Block::Water));
    }
}
