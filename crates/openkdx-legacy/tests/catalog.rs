use std::fs;
use std::path::{Path, PathBuf};

use openkdx_legacy::catalog::{
    CATALOG_FILE, Catalog, CatalogEntry, decode_catalog, encode_catalog, name_mask,
};
use openkdx_legacy::{LegacyDir, LegacyError};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/legacy/lab-2026-11-13/server/catalog")
}

fn fixture(name: &str) -> Vec<u8> {
    fs::read(dir().join(name)).unwrap()
}

#[test]
fn catalog_fixtures_round_trip() {
    for name in [
        "50-catalog-fresh.dat",
        "51-catalog-one.dat",
        "52-catalog-two.dat",
    ] {
        let bytes = fixture(name);
        let c = decode_catalog(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(encode_catalog(&c), bytes, "{name}");
    }
}

#[test]
fn catalog_lists_our_two_files() {
    let c = decode_catalog(&fixture("52-catalog-two.dat")).unwrap();
    assert_eq!(c.version, 1);
    let names: Vec<&[u8]> = c.entries.iter().map(|e| e.name.as_slice()).collect();
    assert_eq!(
        names,
        [
            &b"Bases"[..],
            b"Default",
            b"Admin Dropbox [DB]",
            b"Docs",
            b"Uploads [UL]",
            b"alpha.txt",
            b"beta.txt"
        ]
    );
    let alpha = &c.entries[5];
    assert_eq!(alpha.size, 20);
    assert!(alpha.is_file());
    assert_eq!(alpha.parent, 4);
    assert_eq!(c.entries[6].size, 11);
    let bases = &c.entries[0];
    assert!(!bases.is_file());
    assert_eq!(bases.descendants, 6);
    assert_eq!(c.entries[4].descendants, 2);
    assert_eq!(
        c.path_of(5),
        vec![&b"Bases"[..], b"Default", b"Uploads [UL]", b"alpha.txt"]
    );
    assert_eq!(CATALOG_FILE, "Catalog.dat");
}

#[test]
fn name_mask_matches_the_server_for_every_fixture_name() {
    for name in [
        "50-catalog-fresh.dat",
        "51-catalog-one.dat",
        "52-catalog-two.dat",
    ] {
        let c = decode_catalog(&fixture(name)).unwrap();
        for e in &c.entries {
            assert_eq!(
                name_mask(&e.name),
                e.mask,
                "{:?}",
                String::from_utf8_lossy(&e.name)
            );
        }
    }
    assert_eq!(name_mask(b"Bases"), 0x0004_0013);
    assert_eq!(name_mask(b"BASES"), name_mask(b"bases"));
}

#[test]
fn adding_an_entry_lays_out_like_the_server() {
    // 51 → 52 added `Docs` (index 3) and `beta.txt` (index 6) and bumped
    // counts and times; rebuild 52 from its own entries through the API.
    let want = fixture("52-catalog-two.dat");
    let target = decode_catalog(&want).unwrap();
    let mut c = Catalog {
        entries: Vec::new(),
        ..target.clone()
    };
    for e in &target.entries {
        c.push(CatalogEntry {
            mask: 0,
            ..e.clone()
        });
    }
    assert_eq!(encode_catalog(&c), want);
}

#[test]
fn catalog_errors_and_store() {
    let mut b = fixture("50-catalog-fresh.dat");
    b[0] = b'X';
    assert!(matches!(
        decode_catalog(&b),
        Err(LegacyError::BadMagic { .. })
    ));
    assert!(decode_catalog(&fixture("50-catalog-fresh.dat")[..0x40]).is_err());
    let tmp = tempfile::tempdir().unwrap();
    let store = LegacyDir::open(tmp.path()).unwrap();
    assert!(store.load_catalog().unwrap().is_none());
    let c = decode_catalog(&fixture("51-catalog-one.dat")).unwrap();
    store.save_catalog(&c).unwrap();
    assert_eq!(store.load_catalog().unwrap().unwrap(), c);
}
