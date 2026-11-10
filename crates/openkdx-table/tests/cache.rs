use openkdx_table::{Cache, Story, TableError};

#[test]
fn get_on_empty_cache_is_none() {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::new(dir.path().to_path_buf());
    assert!(cache.get(Story::Kdx).unwrap().is_none());
}

#[test]
fn put_rejects_bytes_with_the_wrong_hash_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::new(dir.path().to_path_buf());
    let bogus = vec![0u8; Story::Kdx.len()];
    assert!(matches!(
        cache.put(Story::Kdx, &bogus),
        Err(TableError::HashMismatch { .. })
    ));
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn get_returns_none_for_a_corrupt_cached_file() {
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::new(dir.path().to_path_buf());
    std::fs::write(dir.path().join(Story::Kdx.file_name()), vec![1u8; 1739]).unwrap();
    assert!(cache.get(Story::Kdx).unwrap().is_none());
}

#[test]
fn put_then_get_roundtrip_with_real_table() {
    let Some(table) = Cache::new(Cache::default_dir())
        .get(Story::Kdx)
        .ok()
        .flatten()
    else {
        eprintln!("skipped: no kdx table available");
        return;
    };
    let dir = tempfile::tempdir().unwrap();
    let cache = Cache::new(dir.path().to_path_buf());
    cache.put(Story::Kdx, &table).unwrap();
    assert_eq!(cache.get(Story::Kdx).unwrap().unwrap(), table);
    assert!(dir.path().join("kdx.bin").is_file());
}
