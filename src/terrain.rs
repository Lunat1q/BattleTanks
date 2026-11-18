//! Destructible terrain: 52 sub-cells (4x4 units) wide, 52 tall on a classic stage, taller in City Run.
//! Pure logic, rendered by `gfx::world`.

use crate::geom::{Dir, FIELD, SUB, SUBS};
use bevy::math::Rect;
use bevy::prelude::Resource;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tile {
    #[default]
    Empty,
    Brick,
    Steel,
    Water,
    Trees,
    Ice,
    /// City Run fortress wall: blocks everything, never breaks.
    Wall,
    /// Fortress gate: like `Wall` until its bastion's turrets are destroyed.
    Gate,
}

impl Tile {
    pub fn blocks_tank(self) -> bool {
        matches!(self, Tile::Brick | Tile::Steel | Tile::Water | Tile::Wall | Tile::Gate)
    }
    pub fn blocks_bullet(self) -> bool {
        matches!(self, Tile::Brick | Tile::Steel | Tile::Wall | Tile::Gate)
    }
    fn unbreakable(self) -> bool {
        matches!(self, Tile::Wall | Tile::Gate)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Impact {
    Brick,
    Steel,
    Edge,
}

#[derive(Resource, Clone)]
pub struct Terrain {
    tiles: Vec<Tile>,
    /// Sub-cell rows (52 on a classic stage).
    rows: usize,
    /// Sub-cells changed since the renderer last looked.
    pub dirty: Vec<(usize, usize)>,
}

impl Default for Terrain {
    fn default() -> Self {
        Terrain::new(SUBS)
    }
}

impl Terrain {
    pub fn new(rows: usize) -> Self {
        Terrain { tiles: vec![Tile::Empty; SUBS * rows], rows, dirty: Vec::new() }
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Height in field units.
    pub fn height(&self) -> f32 {
        self.rows as f32 * SUB
    }

    /// Sub-cell index range `[x0, x1) x [y0, y1)` strictly overlapped by `r`, clamped to the grid.
    fn span(&self, r: Rect) -> (usize, usize, usize, usize) {
        let lo = |v: f32, n: usize| ((v / SUB).floor().max(0.0) as usize).min(n);
        let hi = |v: f32, n: usize| ((v / SUB).ceil().max(0.0) as usize).min(n);
        (lo(r.min.x, SUBS), hi(r.max.x, SUBS), lo(r.min.y, self.rows), hi(r.max.y, self.rows))
    }

    fn outside(&self, r: Rect) -> bool {
        r.min.x < 0.0 || r.min.y < 0.0 || r.max.x > FIELD || r.max.y > self.height()
    }

    /// Every sub-cell, e.g. to mark the whole map dirty for the renderer.
    pub fn cells(&self) -> impl Iterator<Item = (usize, usize)> + use<> {
        let rows = self.rows;
        (0..rows).flat_map(|y| (0..SUBS).map(move |x| (x, y)))
    }

    pub fn get(&self, x: usize, y: usize) -> Tile {
        self.tiles[y * SUBS + x]
    }

    pub fn set(&mut self, x: usize, y: usize, t: Tile) {
        let cell = &mut self.tiles[y * SUBS + x];
        if *cell != t {
            *cell = t;
            self.dirty.push((x, y));
        }
    }

    /// Tile under a field position (for ice checks).
    pub fn at(&self, x: f32, y: f32) -> Tile {
        if !(0.0..FIELD).contains(&x) || !(0.0..self.height()).contains(&y) {
            return Tile::Empty;
        }
        self.get((x / SUB) as usize, (y / SUB) as usize)
    }

    pub fn blocks_tank(&self, r: Rect) -> bool {
        if self.outside(r) {
            return true;
        }
        let (x0, x1, y0, y1) = self.span(r);
        (y0..y1).any(|y| (x0..x1).any(|x| self.get(x, y).blocks_tank()))
    }

    /// Bullet rectangle `r` travelling `dir`. On contact, destroys a 16-wide strip (4 deep, 8 with
    /// `power`, which also breaks steel) and reports what was hit.
    pub fn bullet_hit(&mut self, r: Rect, dir: Dir, power: bool) -> Option<Impact> {
        if self.outside(r) {
            return Some(Impact::Edge);
        }
        let (x0, x1, y0, y1) = self.span(r);
        // Rows (along travel) ordered from the bullet's back to its front.
        let along: Vec<usize> = match dir {
            Dir::Up => (y0..y1).collect(),
            Dir::Down => (y0..y1).rev().collect(),
            Dir::Right => (x0..x1).collect(),
            Dir::Left => (x0..x1).rev().collect(),
        };
        let tile_at = |a: usize, c: usize| if dir.is_vertical() { self.get(c, a) } else { self.get(a, c) };
        let (c0, c1) = if dir.is_vertical() { (x0, x1) } else { (y0, y1) };
        let hit_row = *along.iter().find(|&&a| (c0..c1).any(|c| tile_at(a, c).blocks_bullet()))?;

        let center = if dir.is_vertical() { r.center().x } else { r.center().y };
        let (s0, s1) = if dir.is_vertical() {
            let (a, b, _, _) = self.span(Rect::new(center - 8.0, 0.0, center + 8.0, 1.0));
            (a, b)
        } else {
            let (_, _, a, b) = self.span(Rect::new(0.0, center - 8.0, 1.0, center + 8.0));
            (a, b)
        };
        let steel_hit = (c0..c1).any(|c| tile_at(hit_row, c) == Tile::Steel);
        let hard_hit = (c0..c1).any(|c| tile_at(hit_row, c).unbreakable());

        let step: isize = if matches!(dir, Dir::Up | Dir::Right) { 1 } else { -1 };
        for d in 0..if power { 2 } else { 1 } {
            let a = hit_row as isize + step * d;
            let along_len = if dir.is_vertical() { self.rows } else { SUBS };
            if !(0..along_len as isize).contains(&a) {
                continue;
            }
            for c in s0..s1 {
                let (x, y) = if dir.is_vertical() { (c, a as usize) } else { (a as usize, c) };
                match self.get(x, y) {
                    Tile::Brick => self.set(x, y, Tile::Empty),
                    Tile::Steel if power => self.set(x, y, Tile::Empty),
                    _ => {}
                }
            }
        }
        Some(if hard_hit || (steel_hit && !power) { Impact::Steel } else { Impact::Brick })
    }

    /// Sub-cells of the wall ring around the base (base sits at x 96..112, y 0..16).
    pub fn base_ring() -> impl Iterator<Item = (usize, usize)> {
        (0..6).flat_map(|y| (22..30).map(move |x| (x, y))).filter(|&(x, y)| !((24..28).contains(&x) && y < 4))
    }

    pub fn set_base_ring(&mut self, t: Tile) {
        for (x, y) in Terrain::base_ring() {
            self.set(x, y, t);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::rect_at;
    use bevy::math::Vec2;

    fn wall(t: Tile, rows: std::ops::Range<usize>) -> Terrain {
        let mut ter = Terrain::default();
        for y in rows {
            for x in 0..SUBS {
                ter.set(x, y, t);
            }
        }
        ter
    }

    #[test]
    fn bullet_digs_a_tank_wide_strip() {
        let mut ter = wall(Tile::Brick, 10..14);
        // Bullet moving up at x = 72, its top edge just entering row 10 (y = 40).
        let hit = ter.bullet_hit(rect_at(Vec2::new(72.0, 39.0), 4.0), Dir::Up, false);
        assert_eq!(hit, Some(Impact::Brick));
        for x in 16..20 {
            assert_eq!(ter.get(x, 10), Tile::Empty, "column {x}");
            assert_eq!(ter.get(x, 11), Tile::Brick);
        }
        assert_eq!(ter.get(15, 10), Tile::Brick);
        assert_eq!(ter.get(20, 10), Tile::Brick);
    }

    #[test]
    fn steel_needs_power() {
        let mut ter = wall(Tile::Steel, 10..12);
        let r = rect_at(Vec2::new(72.0, 39.0), 4.0);
        assert_eq!(ter.bullet_hit(r, Dir::Up, false), Some(Impact::Steel));
        assert_eq!(ter.get(17, 10), Tile::Steel);
        assert_eq!(ter.bullet_hit(r, Dir::Up, true), Some(Impact::Brick));
        assert_eq!(ter.get(17, 10), Tile::Empty);
        assert_eq!(ter.get(17, 11), Tile::Empty, "power digs two rows");
    }

    #[test]
    fn water_blocks_tanks_not_bullets() {
        let mut ter = wall(Tile::Water, 10..12);
        assert!(ter.blocks_tank(rect_at(Vec2::new(72.0, 36.0), 16.0)));
        assert_eq!(ter.bullet_hit(rect_at(Vec2::new(72.0, 42.0), 4.0), Dir::Up, false), None);
    }

    #[test]
    fn field_edge_stops_everything() {
        let mut ter = Terrain::default();
        assert!(ter.blocks_tank(rect_at(Vec2::new(7.0, 100.0), 16.0)));
        assert!(!ter.blocks_tank(rect_at(Vec2::new(8.0, 100.0), 16.0)));
        assert_eq!(ter.bullet_hit(rect_at(Vec2::new(100.0, 207.0), 4.0), Dir::Up, false), Some(Impact::Edge));
    }

    #[test]
    fn tall_terrain_and_fortress_walls() {
        let mut ter = Terrain::new(SUBS * 3);
        assert_eq!(ter.height(), FIELD * 3.0);
        assert!(!ter.blocks_tank(rect_at(Vec2::new(100.0, 500.0), 16.0)), "above a classic field is still inside");
        for x in 0..SUBS {
            ter.set(x, 100, Tile::Wall);
            ter.set(x, 101, Tile::Gate);
        }
        // Even a power bullet only sparks off a fortress wall and leaves it standing.
        assert_eq!(ter.bullet_hit(rect_at(Vec2::new(72.0, 399.0), 4.0), Dir::Up, true), Some(Impact::Steel));
        assert_eq!(ter.get(17, 100), Tile::Wall);
        assert_eq!(ter.get(17, 101), Tile::Gate);
        assert!(ter.blocks_tank(rect_at(Vec2::new(72.0, 400.0), 16.0)));
        assert_eq!(ter.bullet_hit(rect_at(Vec2::new(100.0, FIELD * 3.0 - 1.0), 4.0), Dir::Up, false), Some(Impact::Edge));
    }

    #[test]
    fn base_ring_surrounds_base() {
        let ring: Vec<_> = Terrain::base_ring().collect();
        assert_eq!(ring.len(), 8 * 6 - 4 * 4);
        assert!(!ring.contains(&(25, 2)));
        assert!(ring.contains(&(23, 0)) && ring.contains(&(25, 5)));
    }
}
