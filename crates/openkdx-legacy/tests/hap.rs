//! `.hap` appearance files. The only real samples are Haxial's and third
//! parties' artwork, which stay on the lab machine: the tests over them skip
//! unless `OPENKDX_LAB_HAP_DIR` points at a directory tree of `.hap` files.

use std::fs;
use std::path::PathBuf;

use openkdx_legacy::LegacyError;
use openkdx_legacy::hap::{Bitmap, Theme, ThemeInfo, decode_hap, encode_hap};
use openkdx_proto::msg::common::Time1900;

fn lab_files() -> Vec<(PathBuf, Vec<u8>)> {
    let Some(root) = std::env::var_os("OPENKDX_LAB_HAP_DIR") else {
        eprintln!("skipped: OPENKDX_LAB_HAP_DIR not set");
        return Vec::new();
    };
    fn walk(dir: &std::path::Path, out: &mut Vec<(PathBuf, Vec<u8>)>) {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().is_some_and(|e| e == "hap") {
                out.push((p.clone(), fs::read(&p).unwrap()));
            }
        }
    }
    let mut out = Vec::new();
    walk(&PathBuf::from(root), &mut out);
    out.sort();
    assert!(!out.is_empty(), "no .hap files under OPENKDX_LAB_HAP_DIR");
    out
}

fn sample() -> Theme {
    let mut t = Theme::default();
    t.info.name = b"openkdx-test".to_vec();
    t.info.version = b"1.0".to_vec();
    t.info.creator = b"OpenKDX".to_vec();
    t.info.description = b"Every pixel here is ours.".to_vec();
    t.info.created = Time1900::from_unix_ms(1_794_594_187_000);
    t.info.modified = t.info.created;
    t.colors[0] = 0x0011_2233;
    t.colors[203] = 0x00ff_ffff;
    let stripes = Bitmap::new(
        16,
        8,
        1,
        vec![0x0000_0000, 0x00ff_ffff],
        (0..8).flat_map(|_| [0xaa, 0xaa, 0, 0]).collect(),
    )
    .unwrap();
    let four = Bitmap::new(
        5,
        3,
        2,
        vec![0x0000_0000, 0x0055_5555, 0x00aa_aaaa, 0x00ff_ffff],
        vec![0x1b, 0, 0, 0, 0x1b, 0, 0, 0, 0x1b, 0, 0, 0],
    )
    .unwrap();
    let i = t.add_image(stripes.clone(), &[17, 25]).unwrap();
    assert_eq!(i, 0);
    t.add_icon(four, &[4]).unwrap();
    t
}

#[test]
fn constructed_theme_round_trips_and_lays_out_like_the_pack() {
    let t = sample();
    let bytes = encode_hap(&t);
    assert_eq!(&bytes[..6], b"%HAP\x00\x01");
    let again = decode_hap(&bytes).unwrap();
    assert_eq!(again, t);
    assert_eq!(encode_hap(&again), bytes);
    // Sections 4–7 at 0x88 on, images table 528 slots, icons 768 slots.
    assert_eq!(again.table()[4].0, 0x88);
    assert_eq!(again.image_slots.len(), 528);
    assert_eq!(again.icon_slots.len(), 768);
    assert_eq!(again.image_slots[17], Some(0));
    assert_eq!(again.image_slots[25], Some(0));
    assert_eq!(again.icon_slots[4], Some(0));
    assert_eq!(again.colors.len(), 204);
    assert_eq!(again.images[0].pixel(0, 0), 1);
    assert_eq!(again.images[0].pixel(1, 0), 0);
    assert_eq!(again.icons[0].pixel(1, 1), 1);
    assert_eq!(again.icons[0].pixel(2, 1), 2);
}

#[test]
fn bitmap_rgba_uses_the_palette_and_the_transparent_index() {
    let mut b = Bitmap::new(2, 1, 8, vec![0x00ff_0000, 0x0000_ff00], vec![0, 1, 0, 0]).unwrap();
    assert_eq!(b.rgba(), vec![255, 0, 0, 255, 0, 255, 0, 255]);
    b.transparent = Some(1);
    assert_eq!(b.rgba(), vec![255, 0, 0, 255, 0, 255, 0, 0]);
    assert!(
        Bitmap::new(2, 1, 3, vec![0], vec![0; 4]).is_err(),
        "depth must be 1, 2, 4 or 8"
    );
    assert!(
        Bitmap::new(2, 1, 1, vec![0], vec![0; 3]).is_err(),
        "rows are padded to 4 bytes"
    );
    assert!(
        Bitmap::new(2, 1, 1, vec![], vec![0; 4]).is_err(),
        "a palette is needed"
    );
    assert!(
        Bitmap::new(2, 1, 1, vec![0; 300], vec![0; 4]).is_err(),
        "at most 256 colours"
    );
}

#[test]
fn hap_errors() {
    assert!(matches!(
        decode_hap(&[0u8; 0x90]),
        Err(LegacyError::BadMagic { .. })
    ));
    let mut b = encode_hap(&sample());
    b[5] = 2;
    assert!(matches!(
        decode_hap(&b),
        Err(LegacyError::UnsupportedVersion { .. })
    ));
    let b = encode_hap(&sample());
    assert!(decode_hap(&b[..0x200]).is_err());
    let mut t = sample();
    t.info.name = vec![b'n'; 300];
    assert!(t.validate().is_err());
    let _ = ThemeInfo::default();
}

#[test]
fn lab_every_pack_file_round_trips() {
    for (path, bytes) in lab_files() {
        let t = decode_hap(&bytes).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        assert_eq!(encode_hap(&t), bytes, "{path:?}");
        assert_eq!(t.colors.len(), 204, "{path:?}");
        for b in t.images.iter().chain(&t.icons) {
            let _ = b.rgba();
        }
    }
}

#[test]
fn lab_pack_has_the_known_shapes() {
    let files = lab_files();
    if files.is_empty() {
        return;
    }
    let x = files
        .iter()
        .find(|(p, _)| p.ends_with("X.hap"))
        .expect("X.hap in the pack");
    let t = decode_hap(&x.1).unwrap();
    assert_eq!(t.info.name, b"X.hap");
    assert_eq!(t.info.version, b"1.000");
    assert_eq!(t.info.creator, b"resident");
    assert_eq!(t.images.len(), 122);
    assert_eq!(t.image_slots.iter().flatten().count(), 122);
    let first = &t.images[0];
    assert_eq!((first.width, first.height, first.depth), (64, 64, 1));
    let std = files
        .iter()
        .find(|(p, _)| p.ends_with("standard.hap"))
        .expect("standard.hap");
    let t = decode_hap(&std.1).unwrap();
    assert!(t.images.is_empty() && t.icons.is_empty());
    assert_eq!(t.colors[1], 0x00ff_ffff);
}

#[test]
fn decode_only_accepts_what_encode_writes() {
    let base = encode_hap(&sample());
    // An empty section whose table offset is neither 0 nor the running offset.
    let mut b = base.clone();
    b[0x0c + 8 * 9..0x0c + 8 * 9 + 4].copy_from_slice(&0x1234u32.to_be_bytes());
    assert!(decode_hap(&b).is_err(), "stray empty-section offset");
    // A bitmap flag that is neither 0 nor 1.
    let t = decode_hap(&base).unwrap();
    let (img_off, _) = t.table()[5];
    let rec = img_off as usize + 528 * 4;
    let mut b = base.clone();
    b[rec + 4] = 2;
    assert!(decode_hap(&b).is_err(), "bitmap flag 2");
    // A section whose length is not a multiple of 4 (the info section
    // without its padding, the pad bytes left as a gap).
    let mut b = base.clone();
    let len = u32::from_be_bytes(b[0x0c + 8 * 4 + 4..0x0c + 8 * 4 + 8].try_into().unwrap());
    b[0x0c + 8 * 4 + 4..0x0c + 8 * 4 + 8].copy_from_slice(&(len - 1).to_be_bytes());
    assert!(decode_hap(&b).is_err(), "unpadded section");
    // A slot table with no records at all.
    let mut t = sample();
    t.icons.clear();
    t.icon_slots = vec![None; 768];
    let mut b = encode_hap(&t);
    // Forge: make section 7 a 768-slot table of zeros.
    let tail_len = b.len();
    b.extend(std::iter::repeat_n(0u8, 768 * 4));
    b[0x0c + 8 * 7..0x0c + 8 * 7 + 8]
        .copy_from_slice(&[(tail_len as u32).to_be_bytes(), (768u32 * 4).to_be_bytes()].concat());
    assert!(decode_hap(&b).is_err(), "all-zero slot table");
}

#[test]
fn validate_catches_themes_that_would_not_read_back() {
    let good = sample();
    assert!(good.validate().is_ok());
    let mut t = good.clone();
    t.images.push(t.images[0].clone());
    assert!(t.validate().is_err(), "an image no slot uses");
    let mut t = good.clone();
    t.image_slots[3] = Some(9);
    assert!(t.validate().is_err(), "slot past the list");
    let mut t = good.clone();
    t.image_slots.truncate(10);
    assert!(t.validate().is_err(), "short slot table");
    let mut t = good.clone();
    t.other[6] = vec![1];
    assert!(t.validate().is_err(), "bytes in a typed section's raw slot");
}
