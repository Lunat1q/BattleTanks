//! App icon, painted from the game's own tank art. Used at runtime for the window/taskbar icon, and
//! written to `assets/icon.ico` (embedded into the exe by `build.rs`) by:
//! `cargo test write_icon_file -- --ignored`

use super::art_tanks;
use super::canvas::*;
use bevy::ecs::system::NonSendMarker;
use bevy::math::Vec2;
use bevy::prelude::*;

/// Square icon `px` pixels wide: gold top-level tank on a dark rounded tile.
pub fn paint(px: u32) -> Canvas {
    let span = 23.0;
    let ppu = px as f32 / span;
    let mut c = Canvas::new(span, span, ppu);
    let tile = |p: Vec2| sd_rbox(p, Vec2::ZERO, Vec2::splat(11.2), 4.2);
    c.paint(tile, |p| mix(rgb(0.07, 0.09, 0.16), rgb(0.16, 0.22, 0.40), 0.5 + p.y / 23.0));
    c.paint(|p| tile(p).abs() - 0.5, |_| rgb(0.98, 0.80, 0.34));
    let mut tank = art_tanks::hero(ppu, span);
    tank.tint(rgb(1.0, 0.80, 0.32));
    c.over(&tank);
    c
}

/// 5x7 pixel glyphs for the banner title; `#` = lit. Only the letters it needs.
#[cfg(test)]
const GLYPHS: [(char, [&str; 7]); 8] = [
    ('A', [".###.", "#...#", "#...#", "#####", "#...#", "#...#", "#...#"]),
    ('B', ["####.", "#...#", "#...#", "####.", "#...#", "#...#", "####."]),
    ('E', ["#####", "#....", "#....", "####.", "#....", "#....", "#####"]),
    ('K', ["#...#", "#..#.", "#.#..", "##...", "#.#..", "#..#.", "#...#"]),
    ('L', ["#....", "#....", "#....", "#....", "#....", "#....", "#####"]),
    ('N', ["#...#", "##..#", "#.#.#", "#..##", "#...#", "#...#", "#...#"]),
    ('S', [".####", "#....", "#....", ".###.", "....#", "....#", "####."]),
    ('T', ["#####", "..#..", "..#..", "..#..", "..#..", "..#..", "..#.."]),
];

/// Android TV launcher banner, `320k x 180k` px (k = 2 for xhdpi): the icon tile beside "BATTLE / TANKS".
#[cfg(test)]
pub fn banner(k: usize) -> Canvas {
    let mut c = Canvas::new(320.0, 180.0, k as f32);
    c.fill(|p| mix(rgb(0.05, 0.07, 0.13), rgb(0.14, 0.19, 0.34), 0.5 + p.y / 180.0));
    c.blit(&paint(140 * k as u32), 16 * k, 20 * k);
    let cell = 4.0; // banner units per glyph pixel
    for (line, word) in ["BATTLE", "TANKS"].iter().enumerate() {
        let top = 50.0 + line as f32 * 44.0;
        for (n, ch) in word.chars().enumerate() {
            let rows = GLYPHS.iter().find(|g| g.0 == ch).expect("glyph").1;
            for (r, row) in rows.iter().enumerate() {
                for (col, _) in row.chars().enumerate().filter(|(_, b)| *b == '#') {
                    // Canvas origin is the centre, y up.
                    let x = 170.0 + (n * 6 + col) as f32 * cell - 160.0 + cell / 2.0;
                    let y = 90.0 - (top + r as f32 * cell + cell / 2.0);
                    let at = Vec2::new(x, y);
                    c.paint(|p| sd_box(p, at, Vec2::splat(cell / 2.0)), |_| rgb(0.98, 0.80, 0.34));
                }
            }
        }
    }
    c
}

/// ICO file with one 32-bit BMP image per `(size, rgba)` entry. Test-only: it feeds `assets/icon.ico`.
#[cfg(test)]
pub fn ico(images: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&[0, 0, 1, 0]);
    out.extend_from_slice(&(images.len() as u16).to_le_bytes());
    let bmps: Vec<Vec<u8>> = images.iter().map(|(s, rgba)| bmp(*s, rgba)).collect();
    let mut offset = 6 + 16 * images.len() as u32;
    for ((s, _), b) in images.iter().zip(&bmps) {
        let dim = if *s >= 256 { 0 } else { *s as u8 };
        out.extend_from_slice(&[dim, dim, 0, 0]);
        out.extend_from_slice(&1u16.to_le_bytes()); // planes
        out.extend_from_slice(&32u16.to_le_bytes()); // bpp
        out.extend_from_slice(&(b.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += b.len() as u32;
    }
    bmps.iter().for_each(|b| out.extend_from_slice(b));
    out
}

#[cfg(test)]
/// BITMAPINFOHEADER + bottom-up BGRA + empty AND mask (alpha does the masking).
fn bmp(s: u32, rgba: &[u8]) -> Vec<u8> {
    let mask_row = s.div_ceil(32) * 4;
    let mut out = Vec::new();
    for v in [40u32, s, s * 2] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    for v in [0u32, s * s * 4 + mask_row * s, 0, 0, 0, 0] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    for row in (0..s as usize).rev() {
        for px in rgba[row * s as usize * 4..(row + 1) * s as usize * 4].chunks(4) {
            out.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
        }
    }
    out.resize(out.len() + (mask_row * s) as usize, 0);
    out
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, set_window_icon);
}

/// winit icons must be set on the main thread, once the window exists.
fn set_window_icon(_main_thread: NonSendMarker, mut icon: Local<Option<winit::window::Icon>>, mut done: Local<bool>) {
    if *done {
        return;
    }
    let icon = icon.get_or_insert_with(|| {
        let c = paint(64);
        let (w, h) = c.size();
        winit::window::Icon::from_rgba(c.rgba8(), w, h).expect("64x64 RGBA icon")
    });
    bevy::winit::WINIT_WINDOWS.with_borrow(|ww| {
        for w in ww.windows.values() {
            w.set_window_icon(Some(icon.clone()));
            *done = true;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ico_layout() {
        let data = ico(&[(16, vec![255; 16 * 16 * 4]), (256, vec![0; 256 * 256 * 4])]);
        assert_eq!(&data[0..6], &[0, 0, 1, 0, 2, 0]);
        assert_eq!(data[6], 16);
        assert_eq!(data[22], 0, "256 is stored as 0");
        let first_offset = u32::from_le_bytes(data[18..22].try_into().unwrap()) as usize;
        assert_eq!(first_offset, 6 + 32);
        assert_eq!(u32::from_le_bytes(data[first_offset..first_offset + 4].try_into().unwrap()), 40);
    }

    fn save_png(c: &Canvas, path: &str) {
        let (w, h) = c.size();
        let img = Image::new(
            bevy::render::render_resource::Extent3d { width: w, height: h, depth_or_array_layers: 1 },
            bevy::render::render_resource::TextureDimension::D2,
            c.rgba8(),
            bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
            bevy::asset::RenderAssetUsages::all(),
        );
        std::fs::create_dir_all(std::path::Path::new(path).parent().unwrap()).unwrap();
        img.try_into_dynamic().unwrap().save(path).unwrap();
    }

    #[test]
    #[ignore = "writes assets/icon.ico and the Android launcher images; run when the icon art changes"]
    fn write_icon_file() {
        let images: Vec<(u32, Vec<u8>)> = [16, 24, 32, 48, 64, 128, 256].iter().map(|&s| (s, paint(s).rgba8())).collect();
        std::fs::create_dir_all("assets").unwrap();
        std::fs::write("assets/icon.ico", ico(&images)).unwrap();
        // A PNG preview for eyeballing in reviews/screenshots.
        save_png(&paint(256), "assets/icon.png");
        let res = "android/app/src/main/res";
        save_png(&paint(192), &format!("{res}/mipmap-xxxhdpi/ic_launcher.png"));
        save_png(&banner(2), &format!("{res}/drawable-xhdpi/banner.png"));
    }
}
