//! Field geometry: units, directions and rectangle helpers. Pure, no ECS.
//!
//! Field coordinates: origin at the bottom-left corner of the play field, x right, y up.
//! The world `Transform` uses the same coordinates, so `translation.xy()` *is* the field position.

use bevy::math::{Rect, Vec2};

/// Terrain resolution: bricks break in 4x4 pieces.
pub const SUB: f32 = 4.0;
/// One map character = one 8x8 cell (2x2 sub-cells).
pub const CELL: f32 = 8.0;
/// A tank is 16x16 (one "tile").
pub const TILE: f32 = 16.0;
/// Play field is 13x13 tiles.
pub const FIELD: f32 = 208.0;
/// Terrain grid side in sub-cells.
pub const SUBS: usize = 52;
/// Map grid side in cells.
pub const CELLS: usize = 26;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Hash)]
pub enum Dir {
    #[default]
    Up,
    Down,
    Left,
    Right,
}

impl Dir {
    pub const ALL: [Dir; 4] = [Dir::Up, Dir::Down, Dir::Left, Dir::Right];

    pub fn vec(self) -> Vec2 {
        match self {
            Dir::Up => Vec2::Y,
            Dir::Down => Vec2::NEG_Y,
            Dir::Left => Vec2::NEG_X,
            Dir::Right => Vec2::X,
        }
    }

    pub fn is_vertical(self) -> bool {
        matches!(self, Dir::Up | Dir::Down)
    }

    /// Rotation (radians, counter-clockwise) for a sprite drawn facing up.
    pub fn angle(self) -> f32 {
        use std::f32::consts::{FRAC_PI_2, PI};
        match self {
            Dir::Up => 0.0,
            Dir::Left => FRAC_PI_2,
            Dir::Down => PI,
            Dir::Right => -FRAC_PI_2,
        }
    }

    /// Dominant direction of a vector, `None` inside the dead zone.
    pub fn from_vec(v: Vec2, dead_zone: f32) -> Option<Dir> {
        if v.length() < dead_zone {
            return None;
        }
        Some(if v.x.abs() > v.y.abs() {
            if v.x > 0.0 { Dir::Right } else { Dir::Left }
        } else if v.y > 0.0 {
            Dir::Up
        } else {
            Dir::Down
        })
    }
}

pub fn rect_at(center: Vec2, size: f32) -> Rect {
    Rect::from_center_size(center, Vec2::splat(size))
}

/// Strict overlap: rectangles that only touch do not overlap.
pub fn overlaps(a: Rect, b: Rect) -> bool {
    a.min.x < b.max.x && b.min.x < a.max.x && a.min.y < b.max.y && b.min.y < a.max.y
}

/// Snap to the nearest multiple of `CELL` (classic grid alignment on turns).
pub fn snap(v: f32) -> f32 {
    (v / CELL).round() * CELL
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn touching_is_not_overlap() {
        let a = rect_at(Vec2::new(8.0, 8.0), 16.0);
        assert!(!overlaps(a, rect_at(Vec2::new(24.0, 8.0), 16.0)));
        assert!(overlaps(a, rect_at(Vec2::new(23.9, 8.0), 16.0)));
    }

    #[test]
    fn stick_picks_dominant_axis() {
        assert_eq!(Dir::from_vec(Vec2::new(0.9, 0.3), 0.5), Some(Dir::Right));
        assert_eq!(Dir::from_vec(Vec2::new(-0.2, -0.8), 0.5), Some(Dir::Down));
        assert_eq!(Dir::from_vec(Vec2::new(0.2, 0.2), 0.5), None);
    }

    #[test]
    fn snap_rounds_to_cell() {
        assert_eq!(snap(11.9), 8.0);
        assert_eq!(snap(12.1), 16.0);
    }
}
