use std::io::Write;

use openkdx_table::{Cache, Story, TableError, import};

fn zip_with(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut zw = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in entries {
            zw.start_file(*name, opts).unwrap();
            zw.write_all(data).unwrap();
        }
        zw.finish().unwrap();
    }
    buf.into_inner()
}

#[test]
fn import_junk_file_errors_without_writing_cache() {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::new(dir.path().join("cache"));
    let junk = dir.path().join("junk.bin");
    std::fs::write(&junk, vec![0x5Au8; 10_000]).unwrap();
    assert!(matches!(
        import(&junk, &cache),
        Err(TableError::AnchorNotFound)
    ));
    assert!(!dir.path().join("cache").exists());
}

#[test]
fn import_zip_of_junk_errors_without_writing_cache() {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::new(dir.path().join("cache"));
    let z = dir.path().join("junk.zip");
    std::fs::write(
        &z,
        zip_with(&[("a/b.exe", &[0u8; 5000]), ("c.txt", b"hello")]),
    )
    .unwrap();
    assert!(matches!(
        import(&z, &cache),
        Err(TableError::AnchorNotFound)
    ));
    assert!(!dir.path().join("cache").exists());
}

#[test]
fn import_from_zip_finds_real_story() {
    let Some(table) = Cache::new(Cache::default_dir())
        .get(Story::Kdx)
        .ok()
        .flatten()
    else {
        eprintln!("skipped: no kdx table available");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::new(dir.path().join("cache"));
    let blob = [&[0u8; 777][..], &table, &[0xFFu8; 99][..]].concat();
    let z = dir.path().join("prog.zip");
    std::fs::write(
        &z,
        zip_with(&[("Prog/readme.txt", b"x"), ("Prog/prog.exe", &blob)]),
    )
    .unwrap();
    assert_eq!(import(&z, &cache).unwrap(), vec![Story::Kdx]);
    assert_eq!(cache.get(Story::Kdx).unwrap().unwrap(), table);
}
