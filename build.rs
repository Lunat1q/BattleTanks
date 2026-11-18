//! Embeds `assets/icon.ico` as the exe icon (Explorer, Steam shortcut) without extra crates:
//! wraps the ICO images into a Win32 `.res` file and hands it to the MSVC linker.
//! Regenerate the icon with `cargo test write_icon_file -- --ignored`.

use std::{env, fs, path::PathBuf};

const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;

fn le16(v: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([v[at], v[at + 1]])
}
fn le32(v: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([v[at], v[at + 1], v[at + 2], v[at + 3]])
}

/// One resource entry (numeric type and name), data padded to 4 bytes.
fn entry(out: &mut Vec<u8>, kind: u16, name: u16, flags: u16, lang: u16, data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_le_bytes());
    out.extend_from_slice(&32u32.to_le_bytes());
    for v in [0xFFFF, kind, 0xFFFF, name] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(&0u32.to_le_bytes()); // data version
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&lang.to_le_bytes());
    out.extend_from_slice(&[0; 8]); // version, characteristics
    out.extend_from_slice(data);
    out.resize(out.len().next_multiple_of(4), 0);
}

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }
    let ico = fs::read("assets/icon.ico").expect("assets/icon.ico (cargo test write_icon_file -- --ignored)");
    let count = le16(&ico, 4) as usize;
    let mut res = Vec::new();
    entry(&mut res, 0, 0, 0, 0, &[]); // mandatory empty first entry
    let mut group = vec![0, 0, 1, 0];
    group.extend_from_slice(&(count as u16).to_le_bytes());
    for i in 0..count {
        let d = 6 + i * 16;
        let (size, offset) = (le32(&ico, d + 8) as usize, le32(&ico, d + 12) as usize);
        entry(&mut res, RT_ICON, i as u16 + 1, 0x1010, 0x0409, &ico[offset..offset + size]);
        group.extend_from_slice(&ico[d..d + 12]); // w, h, colours, reserved, planes, bpp, bytes
        group.extend_from_slice(&(i as u16 + 1).to_le_bytes());
    }
    entry(&mut res, RT_GROUP_ICON, 1, 0x1030, 0x0409, &group);
    let path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("icon.res");
    fs::write(&path, res).unwrap();
    println!("cargo:rustc-link-arg-bins={}", path.display());
}
