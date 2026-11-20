//! `load`: a `kdx_proto::ChallengeTable` from the environment or the cache.

use openkdx_table::{Story, TableError, load, load_from};

static ENV: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn with_env<T>(key: &str, value: Option<&str>, f: impl FnOnce() -> T) -> T {
    // The variable is process-wide; one test at a time touches it.
    let _guard = ENV.lock().unwrap_or_else(|e| e.into_inner());
    let old = std::env::var(key).ok();
    match value {
        Some(v) => unsafe { std::env::set_var(key, v) },
        None => unsafe { std::env::remove_var(key) },
    }
    let out = f();
    match old {
        Some(v) => unsafe { std::env::set_var(key, v) },
        None => unsafe { std::env::remove_var(key) },
    }
    out
}

#[test]
fn load_default_sentinel_returns_the_default_table() {
    with_env("OPENKDX_HGC_TABLE", Some("default"), || {
        let table = load(Story::Gconsole).unwrap();
        assert_eq!(table.story(), None);
        assert_eq!(table.offsets(), 12, "the 31-byte built-in line");
    });
}

#[test]
fn load_rejects_a_table_with_a_wrong_checksum() {
    use base64::Engine;
    let junk = base64::engine::general_purpose::STANDARD.encode(vec![7u8; 1739]);
    with_env("OPENKDX_HGC_TABLE", Some(&junk), || {
        let err = load(Story::Gconsole).unwrap_err();
        assert!(matches!(err, TableError::HashMismatch { .. }), "{err}");
    });
}

#[test]
fn load_without_env_or_cache_is_no_source() {
    let dir = tempfile::tempdir().unwrap();
    with_env("OPENKDX_HGC_TABLE", None, || {
        let err = load_from(
            Story::Gconsole,
            &openkdx_table::Cache::new(dir.path().to_path_buf()),
        )
        .unwrap_err();
        assert!(
            matches!(err, TableError::NoSource(Story::Gconsole)),
            "{err}"
        );
    });
}
