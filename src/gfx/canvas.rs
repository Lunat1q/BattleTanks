//! Tiny anti-aliased software painter: shapes are signed-distance functions in field units,
//! rendered into an RGBA image at `ppu` pixels per unit. All game art is drawn with this at startup,
//! so the exe ships with no asset files.

use bevy::asset::RenderAssetUsages;
use bevy::math::Vec2;
use bevy::prelude::Image;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Straight (non-premultiplied) sRGB colour with alpha.
pub type C = [f32; 4];

pub const fn rgb(r: f32, g: f32, b: f32) -> C {
    [r, g, b, 1.0]
}
pub fn gray(v: f32) -> C {
    [v, v, v, 1.0]
}
pub fn alpha(c: C, a: f32) -> C {
    [c[0], c[1], c[2], c[3] * a]
}
pub fn shade(c: C, k: f32) -> C {
    [c[0] * k, c[1] * k, c[2] * k, c[3]]
}
pub fn mix(a: C, b: C, t: f32) -> C {
    let t = t.clamp(0.0, 1.0);
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

pub struct Canvas {
    w: usize,
    h: usize,
    ppu: f32,
    half: Vec2,
    /// Premultiplied RGBA.
    px: Vec<[f32; 4]>,
}

impl Canvas {
    /// Canvas `units_w` x `units_h`, origin at the centre, y up.
    pub fn new(units_w: f32, units_h: f32, ppu: f32) -> Self {
        let (w, h) = ((units_w * ppu).round() as usize, (units_h * ppu).round() as usize);
        Canvas { w, h, ppu, half: Vec2::new(units_w, units_h) / 2.0, px: vec![[0.0; 4]; w * h] }
    }

    fn pos(&self, i: usize, j: usize) -> Vec2 {
        Vec2::new((i as f32 + 0.5) / self.ppu - self.half.x, self.half.y - (j as f32 + 0.5) / self.ppu)
    }

    /// Paint where `sdf < 0` (anti-aliased edge) with `color(p)`, alpha-over.
    pub fn paint(&mut self, sdf: impl Fn(Vec2) -> f32, color: impl Fn(Vec2) -> C) {
        for j in 0..self.h {
            for i in 0..self.w {
                let p = self.pos(i, j);
                let cover = (0.5 - sdf(p) * self.ppu).clamp(0.0, 1.0);
                if cover <= 0.0 {
                    continue;
                }
                let c = color(p);
                let a = c[3] * cover;
                let d = &mut self.px[j * self.w + i];
                for k in 0..3 {
                    d[k] = c[k] * a + d[k] * (1.0 - a);
                }
                d[3] = a + d[3] * (1.0 - a);
            }
        }
    }

    pub fn fill(&mut self, color: impl Fn(Vec2) -> C) {
        self.paint(|_| -1.0, color);
    }

    /// Paint the shape with a dark outline of `width` units around it.
    pub fn outlined(&mut self, sdf: impl Fn(Vec2) -> f32, width: f32, color: impl Fn(Vec2) -> C) {
        self.paint(|p| sdf(p) - width, |_| [0.04, 0.04, 0.05, 0.9]);
        self.paint(sdf, color);
    }

    pub fn into_image(self) -> Image {
        let data = self
            .px
            .iter()
            .flat_map(|p| {
                let a = p[3];
                let un = |v: f32| if a > 0.0 { (v / a).clamp(0.0, 1.0) } else { 0.0 };
                [un(p[0]), un(p[1]), un(p[2]), a.clamp(0.0, 1.0)].map(|v| (v * 255.0 + 0.5) as u8)
            })
            .collect();
        Image::new(
            Extent3d { width: self.w as u32, height: self.h as u32, depth_or_array_layers: 1 },
            TextureDimension::D2,
            data,
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        )
    }
}

// ---- signed distance helpers (units; negative inside) ----

pub fn sd_box(p: Vec2, c: Vec2, half: Vec2) -> f32 {
    let d = (p - c).abs() - half;
    d.max(Vec2::ZERO).length() + d.x.max(d.y).min(0.0)
}

pub fn sd_rbox(p: Vec2, c: Vec2, half: Vec2, r: f32) -> f32 {
    sd_box(p, c, half - Vec2::splat(r)) - r
}

pub fn sd_circle(p: Vec2, c: Vec2, r: f32) -> f32 {
    p.distance(c) - r
}

/// Capsule from `a` to `b` with radius `r`.
pub fn sd_seg(p: Vec2, a: Vec2, b: Vec2, r: f32) -> f32 {
    let (pa, ba) = (p - a, b - a);
    let h = (pa.dot(ba) / ba.dot(ba)).clamp(0.0, 1.0);
    (pa - ba * h).length() - r
}

/// Any simple polygon (Inigo Quilez's formulation).
pub fn sd_poly(p: Vec2, v: &[Vec2]) -> f32 {
    let mut d = (p - v[0]).length_squared();
    let mut s = 1.0;
    let mut j = v.len() - 1;
    for i in 0..v.len() {
        let e = v[j] - v[i];
        let w = p - v[i];
        let b = w - e * (w.dot(e) / e.dot(e)).clamp(0.0, 1.0);
        d = d.min(b.length_squared());
        let c = [p.y >= v[i].y, p.y < v[j].y, e.x * w.y > e.y * w.x];
        if c.iter().all(|&x| x) || c.iter().all(|&x| !x) {
            s = -s;
        }
        j = i;
    }
    s * d.sqrt()
}

/// Star outline with `n` points.
pub fn star(c: Vec2, outer: f32, inner: f32, n: usize) -> Vec<Vec2> {
    (0..n * 2)
        .map(|i| {
            let a = std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / n as f32;
            c + Vec2::new(a.cos(), a.sin()) * if i % 2 == 0 { outer } else { inner }
        })
        .collect()
}

// ---- noise ----

pub fn hash(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841) ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    (h & 0xffff) as f32 / 65535.0
}

/// Smooth value noise in [0, 1], tileable with period `period` (in noise cells).
pub fn noise(p: Vec2, seed: u32, period: i32) -> f32 {
    let (i, f) = (p.floor(), p - p.floor());
    let u = f * f * (Vec2::splat(3.0) - 2.0 * f);
    let at = |dx: i32, dy: i32| hash((i.x as i32 + dx).rem_euclid(period), (i.y as i32 + dy).rem_euclid(period), seed);
    let (a, b, c, d) = (at(0, 0), at(1, 0), at(0, 1), at(1, 1));
    a + (b - a) * u.x + (c - a) * u.y + (a - b - c + d) * u.x * u.y
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdf_signs() {
        assert!(sd_box(Vec2::ZERO, Vec2::ZERO, Vec2::ONE) < 0.0);
        assert!((sd_box(Vec2::new(2.0, 0.0), Vec2::ZERO, Vec2::ONE) - 1.0).abs() < 1e-5);
        let tri = [Vec2::new(0.0, 1.0), Vec2::new(-1.0, -1.0), Vec2::new(1.0, -1.0)];
        assert!(sd_poly(Vec2::ZERO, &tri) < 0.0);
        assert!(sd_poly(Vec2::new(0.0, 3.0), &tri) > 0.0);
    }

    #[test]
    fn painting_covers_inside_only() {
        let mut c = Canvas::new(4.0, 4.0, 4.0);
        c.paint(|p| sd_circle(p, Vec2::ZERO, 1.0), |_| gray(1.0));
        let img = c.into_image();
        let data = img.data.unwrap();
        let alpha_at = |i: usize, j: usize| data[(j * 16 + i) * 4 + 3];
        assert_eq!(alpha_at(8, 8), 255);
        assert_eq!(alpha_at(0, 0), 0);
    }

    #[test]
    fn noise_tiles() {
        let a = noise(Vec2::new(0.3, 0.7), 1, 8);
        let b = noise(Vec2::new(8.3, 0.7), 1, 8);
        assert!((a - b).abs() < 1e-5);
    }
}
