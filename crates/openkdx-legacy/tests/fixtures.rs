//! Round trips over every committed fixture: `encode(decode(file)) == file`.

use std::fs;
use std::path::{Path, PathBuf};

use openkdx_legacy::server_stg::{decode_server_stg, encode_server_stg};
use openkdx_legacy::{LegacyDir, LegacyError, ServerFile};
use openkdx_proto::DecodeError;

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/legacy")
}

/// Every file under `fixtures/legacy` whose name matches `pred`, recursively.
fn files(pred: impl Fn(&Path) -> bool + Copy) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>, pred: &dyn Fn(&Path) -> bool) {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.is_dir() {
                walk(&p, out, pred);
            } else if pred(&p) {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    walk(&fixtures(), &mut out, &pred);
    out.sort();
    assert!(!out.is_empty(), "no fixtures matched");
    out
}

fn stg_files() -> Vec<PathBuf> {
    files(|p| {
        p.extension().is_some_and(|e| e == "stg")
            && p.components().any(|c| c.as_os_str() == "server")
    })
}

fn fixture(name: &str) -> Vec<u8> {
    fs::read(fixtures().join("lab-2026-11-13/server/stg").join(name)).unwrap()
}

#[test]
fn every_server_stg_fixture_round_trips() {
    let list = stg_files();
    assert!(list.len() >= 18, "{list:?}");
    for path in list {
        let bytes = fs::read(&path).unwrap();
        let s = decode_server_stg(&bytes).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        assert_eq!(encode_server_stg(&s), bytes, "{path:?}");
    }
}

#[test]
fn stg_wrong_magic_is_bad_magic() {
    let mut b = fixture("00-fresh.stg");
    b[5] = b'X';
    assert!(matches!(
        decode_server_stg(&b),
        Err(LegacyError::BadMagic { .. })
    ));
}

#[test]
fn stg_truncated_is_error() {
    let b = fixture("00-fresh.stg");
    assert!(matches!(
        decode_server_stg(&b[..0x40]),
        Err(LegacyError::Decode(DecodeError::Truncated { .. }))
    ));
    // A table that points past the end.
    let mut b = fixture("00-fresh.stg");
    b[0x10..0x14].copy_from_slice(&0x1000u32.to_be_bytes());
    assert!(matches!(decode_server_stg(&b), Err(LegacyError::Decode(_))));
}

#[test]
fn stg_version_2_is_unsupported() {
    let mut b = fixture("00-fresh.stg");
    b[9] = 2;
    assert!(matches!(
        decode_server_stg(&b),
        Err(LegacyError::UnsupportedVersion {
            file: "KDXServer.stg",
            version: 2
        })
    ));
}

#[test]
fn set_name_recomputes_length_and_pad() {
    let mut s = decode_server_stg(&fixture("00-fresh.stg")).unwrap();
    assert_eq!(s.main.name, b"New Server");
    s.main.name = vec![b'n'; 63];
    let bytes = encode_server_stg(&s);
    assert_eq!(bytes[0x70 + 0x52], 63);
    let again = decode_server_stg(&bytes).unwrap();
    assert_eq!(again.main.name.len(), 63);
    // 0x5c + 63 = 0x9b, padded to 0x9c.
    assert_eq!(again.table()[0].1, 0x9c);
    assert_eq!(again.main.padding, [] as [u8; 0]);
    assert_eq!(encode_server_stg(&again), bytes);
}

#[test]
fn stg_fields_pinned_by_the_lab_diffs() {
    let fresh = decode_server_stg(&fixture("00-fresh.stg")).unwrap();
    assert_eq!(fresh.main.port, 10700);
    assert_eq!(fresh.main.alternate_port, 0);
    assert_eq!(fresh.main.file_transfer_port, 0);
    assert_eq!(fresh.main.history_kinds, 0);
    assert_eq!(fresh.main.history_max_file_size, 0x10_0000);
    assert!(!fresh.main.greeting_enabled());
    assert!(!fresh.main.register_with_trackers());
    assert_eq!(fresh.main.description, b"");
    assert_eq!(fresh.main.address, b"");
    assert!(fresh.greeting.is_none());
    assert!(fresh.icon.is_none());
    assert!(fresh.trackers.is_empty());
    assert!(fresh.allow_deny.is_empty());
    assert_eq!(fresh.chats.len(), 0x104);

    let s = decode_server_stg(&fixture("01-name.stg")).unwrap();
    assert_eq!(s.main.name, b"Legacy Lab");
    let s = decode_server_stg(&fixture("02-description.stg")).unwrap();
    assert_eq!(s.main.description, b"A description");
    let s = decode_server_stg(&fixture("03-address.stg")).unwrap();
    assert_eq!(s.main.address, b"lab.example");
    let s = decode_server_stg(&fixture("04-speed.stg")).unwrap();
    assert_eq!(s.main.total_outgoing_speed_limit, 20 * 1024);
    let s = decode_server_stg(&fixture("05-altport.stg")).unwrap();
    assert_eq!(s.main.alternate_port, 10701);
    let s = decode_server_stg(&fixture("06-greeting.stg")).unwrap();
    assert!(s.main.greeting_enabled());
    assert_eq!(s.greeting.as_ref().unwrap().text, b"Welcome to the lab");
    let s = decode_server_stg(&fixture("07-history-login.stg")).unwrap();
    assert_eq!(s.main.history_kinds, 1);
    let s = decode_server_stg(&fixture("08-history-disc.stg")).unwrap();
    assert_eq!(s.main.history_kinds, 3);
    let s = decode_server_stg(&fixture("09-history-size.stg")).unwrap();
    assert_eq!(s.main.history_max_file_size, 0x8_0000);
    let s = decode_server_stg(&fixture("10-deny.stg")).unwrap();
    assert_eq!(s.allow_deny.len(), 1);
    assert_eq!(s.allow_deny[0].address, b"10.9.8.7");
    assert_eq!(s.allow_deny[0].name, b"Bad host");
    assert!(!s.allow_deny[0].allow);
    assert_eq!(s.allow_deny[0].expires.0, 0);
    let s = decode_server_stg(&fixture("11-allow-expiry.stg")).unwrap();
    assert_eq!(s.allow_deny.len(), 2);
    assert!(s.allow_deny[1].allow);
    assert_eq!(s.allow_deny[1].name, b"Good host");
    // 13 Nov 2026, in ms since 1900.
    let unix_ms = s.allow_deny[1].expires.to_unix_ms();
    assert!(
        (1_794_000_000_000..1_795_000_000_000).contains(&unix_ms),
        "{unix_ms}"
    );
    let s = decode_server_stg(&fixture("12-tracker.stg")).unwrap();
    assert_eq!(s.trackers.len(), 1);
    assert_eq!(s.trackers[0].name, b"Lab tracker");
    assert_eq!(s.trackers[0].address, b"127.0.0.1");
    let s = decode_server_stg(&fixture("13-register.stg")).unwrap();
    assert!(s.main.register_with_trackers());
    let s = decode_server_stg(&fixture("14-ftport.stg")).unwrap();
    assert_eq!(s.main.file_transfer_port, 10702);
    let s = decode_server_stg(&fixture("15-tracker-full.stg")).unwrap();
    let t = &s.trackers[1];
    assert_eq!(t.name, b"Second");
    assert_eq!(t.comments, b"tracker comment");
    assert_eq!(t.address, b"10.0.0.2");
    assert_eq!(t.login, b"tlogin");
    assert_eq!(t.groups, b"grp");
    let s = decode_server_stg(&fixture("16-deny-comment.stg")).unwrap();
    assert_eq!(s.allow_deny[1].comments, b"a comment");
    assert_eq!(s.allow_deny[1].name, b"Noted");
}

#[test]
fn stg_random_ignored_bytes_round_trip() {
    for path in stg_files() {
        let bytes = fs::read(&path).unwrap();
        let mut s = decode_server_stg(&bytes).unwrap();
        for b in s
            .main
            .ignored_00
            .iter_mut()
            .chain(s.main.ignored_3c.iter_mut())
        {
            *b ^= 0x5a;
        }
        for t in &mut s.trackers {
            t.ignored_00[3] ^= 0x5a;
        }
        for r in &mut s.allow_deny {
            r.ignored_08[0] ^= 0x5a;
        }
        let once = encode_server_stg(&s);
        let again = encode_server_stg(&decode_server_stg(&once).unwrap());
        assert_eq!(once, again, "{path:?}");
        assert_ne!(once, bytes);
    }
}

#[test]
fn edits_relayout_sections_like_the_server() {
    // Adding a tracker to the fresh file must produce the same bytes the
    // server wrote when the client added it (12-tracker), modulo the fields
    // that changed in between (name, description, …): so start from 11 and
    // compare to 12 after dropping the expired allow entry.
    let mut s = decode_server_stg(&fixture("11-allow-expiry.stg")).unwrap();
    let want = decode_server_stg(&fixture("12-tracker.stg")).unwrap();
    s.allow_deny.truncate(1);
    s.trackers.push(want.trackers[0].clone());
    assert_eq!(encode_server_stg(&s), fixture("12-tracker.stg"));
}

#[test]
fn legacy_dir_loads_and_saves_settings() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("KDXServer.stg"), fixture("00-fresh.stg")).unwrap();
    let store = LegacyDir::open(dir.path()).unwrap();
    let mut s = store.load_settings().unwrap().expect("present");
    s.main.name = b"Saved".to_vec();
    store.save_settings(&s).unwrap();
    assert_eq!(store.load_settings().unwrap().unwrap().main.name, b"Saved");
    assert_eq!(
        fs::read(dir.path().join("KDXServer.stg.orig")).unwrap(),
        fixture("00-fresh.stg")
    );
    let empty = tempfile::tempdir().unwrap();
    assert!(
        LegacyDir::open(empty.path())
            .unwrap()
            .load_settings()
            .unwrap()
            .is_none()
    );
    let _ = ServerFile::Settings;
}

#[test]
fn overlong_string_is_rejected_by_validate_and_never_corrupts_layout() {
    let mut s = decode_server_stg(&fixture("03-address.stg")).unwrap();
    s.main.description = vec![b'd'; 300];
    assert!(matches!(s.validate(), Err(DecodeError::Invalid(_))));
    // The encoder writes what the length byte says, so a decode still finds
    // the address where it belongs.
    let again = decode_server_stg(&encode_server_stg(&s)).unwrap();
    assert_eq!(again.main.description.len(), 255);
    assert_eq!(again.main.address, b"lab.example");
    let dir = tempfile::tempdir().unwrap();
    let store = LegacyDir::open(dir.path()).unwrap();
    assert!(matches!(
        store.save_settings(&s),
        Err(LegacyError::Decode(_))
    ));
    assert!(!dir.path().join("KDXServer.stg").exists());
}

#[test]
fn trailing_bytes_after_the_last_section_survive() {
    let mut b = fixture("00-fresh.stg");
    b.extend_from_slice(&[7, 7, 7, 7, 7]);
    let s = decode_server_stg(&b).unwrap();
    assert_eq!(s.trailing, [7, 7, 7, 7, 7]);
    assert_eq!(encode_server_stg(&s), b);
}

#[test]
fn unpinned_length_bytes_refuse_to_load() {
    // main 0x54 sits between the description and address length bytes.
    let mut b = fixture("00-fresh.stg");
    b[0x70 + 0x54] = 3;
    assert!(matches!(
        decode_server_stg(&b),
        Err(LegacyError::Decode(DecodeError::Invalid(_)))
    ));
    // tracker entry 0x18 likewise.
    let mut b = fixture("12-tracker.stg");
    b[0xf0 + 0x18] = 1;
    assert!(matches!(
        decode_server_stg(&b),
        Err(LegacyError::Decode(DecodeError::Invalid(_)))
    ));
}

#[test]
fn high_bytes_and_cleared_strings_recompute_lengths() {
    let mut s = decode_server_stg(&fixture("16-deny-comment.stg")).unwrap();
    s.main.name = vec![0xa5, 0xff, b'x'];
    s.main.description.clear();
    s.main.address.clear();
    let bytes = encode_server_stg(&s);
    assert_eq!(bytes[0x70 + 0x52..0x70 + 0x56], [3, 0, 0, 0]);
    assert_eq!(s.table()[0].1, 0x60);
    let again = decode_server_stg(&bytes).unwrap();
    assert_eq!(again.main.name, [0xa5, 0xff, b'x']);
    assert_eq!(again.allow_deny, s.allow_deny);
}
