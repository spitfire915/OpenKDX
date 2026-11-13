use std::fs;

use openkdx_legacy::{LegacyDir, ServerFile};

#[test]
fn open_makes_orig_once_and_never_replaces_it() {
    let dir = tempfile::tempdir().unwrap();
    let live = dir.path().join("Accounts.dat");
    fs::write(&live, b"v1").unwrap();

    LegacyDir::open(dir.path()).unwrap();
    assert_eq!(
        fs::read(dir.path().join("Accounts.dat.orig")).unwrap(),
        b"v1"
    );

    fs::write(&live, b"v2").unwrap();
    LegacyDir::open(dir.path()).unwrap();
    assert_eq!(
        fs::read(dir.path().join("Accounts.dat.orig")).unwrap(),
        b"v1"
    );
}

#[test]
fn open_does_not_write_when_nothing_to_back_up() {
    let dir = tempfile::tempdir().unwrap();
    LegacyDir::open(dir.path()).unwrap();
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}

#[test]
fn open_fails_on_missing_root() {
    let dir = tempfile::tempdir().unwrap();
    assert!(LegacyDir::open(dir.path().join("nope")).is_err());
}

#[test]
fn save_is_atomic_and_replaces_stale_tmp() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("KDXServer.stg.tmp"), b"stale junk").unwrap();
    let store = LegacyDir::open(dir.path()).unwrap();

    store.save(ServerFile::Settings, b"fresh").unwrap();

    assert_eq!(
        fs::read(dir.path().join("KDXServer.stg")).unwrap(),
        b"fresh"
    );
    assert!(!dir.path().join("KDXServer.stg.tmp").exists());
    assert_eq!(store.read(ServerFile::Settings).unwrap().unwrap(), b"fresh");
    assert_eq!(store.read(ServerFile::News).unwrap(), None);
}

#[cfg(unix)]
#[test]
fn save_keeps_mode_0600() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let live = dir.path().join("Accounts.dat");
    fs::write(&live, b"old").unwrap();
    fs::set_permissions(&live, fs::Permissions::from_mode(0o600)).unwrap();
    let store = LegacyDir::open(dir.path()).unwrap();

    store.save(ServerFile::Accounts, b"new").unwrap();
    assert_eq!(
        fs::metadata(&live).unwrap().permissions().mode() & 0o777,
        0o600
    );

    // A file that did not exist before is created 0600 too.
    store.save(ServerFile::News, b"n").unwrap();
    let m = fs::metadata(dir.path().join("News.dat")).unwrap();
    assert_eq!(m.permissions().mode() & 0o777, 0o600);
}

#[test]
fn file_names_and_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let store = LegacyDir::open(dir.path()).unwrap();
    assert_eq!(ServerFile::Accounts.name(), "Accounts.dat");
    assert_eq!(ServerFile::Settings.name(), "KDXServer.stg");
    assert_eq!(ServerFile::News.name(), "News.dat");
    assert_eq!(ServerFile::Tracker.name(), "KDXTrackerServ.stg");
    assert_eq!(store.path(ServerFile::News), dir.path().join("News.dat"));
    assert_eq!(store.history_dir(), dir.path().join("History"));
    assert_eq!(store.news_archives_dir(), dir.path().join("News Archives"));
    assert_eq!(store.bases_dir(), dir.path().join("Bases"));
    assert_eq!(store.root(), dir.path());
}
