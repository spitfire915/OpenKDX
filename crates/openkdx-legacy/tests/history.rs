use std::fs;
use std::path::{Path, PathBuf};

use openkdx_legacy::LegacyDir;
use openkdx_legacy::history::{
    HistoryFile, HistoryLine, format_history_time, history_file_name, parse_history,
};
use openkdx_legacy::news::decode_news;
use openkdx_legacy::news_archive::{
    ArchiveEntry, archive_file_name, format_archive_entry, format_archive_time, parse_news_archive,
};
use openkdx_proto::msg::common::Time1900;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/legacy/lab-2026-11-13/server")
}

fn history_fixture() -> Vec<u8> {
    fs::read(root().join("History/KDX 26-11-13 06-23-07 PM.txt")).unwrap()
}

fn archive_fixture() -> Vec<u8> {
    fs::read(root().join("News Archives/Second Group/Second Group 261113.txt")).unwrap()
}

#[test]
fn history_fixture_lines_round_trip_byte_exact() {
    let bytes = history_fixture();
    let f = parse_history(&bytes).unwrap();
    assert!(f.bom);
    assert_eq!(f.lines.len(), bytes.iter().filter(|&&b| b == b'\n').count());
    assert_eq!(f.to_bytes(), bytes);
}

#[test]
fn history_login_line_has_event_login_and_admin() {
    let f = parse_history(&history_fixture()).unwrap();
    let login = f
        .lines
        .iter()
        .find(|l| l.event == b"Login")
        .expect("a Login line");
    assert_eq!(login.login, b"admin");
    assert_eq!(login.nick, b"openkdx");
    assert_eq!(login.time, b"26/11/13 06:23:09 PM");
    assert!(login.detail.starts_with(b"127.0.0.1:"));
    assert_eq!(login.extra, vec![b"0".to_vec()]);
    let up = f
        .lines
        .iter()
        .find(|l| l.event == b"Finished File Upload")
        .unwrap();
    assert_eq!(up.detail, b"small.txt");
    assert_eq!(
        up.extra,
        vec![b"0".to_vec(), b"11".to_vec(), b"11".to_vec()]
    );
    let created = f
        .lines
        .iter()
        .find(|l| l.event == b"Create Account")
        .unwrap();
    assert_eq!(created.detail, b"hist1");
}

#[test]
fn history_time_and_file_name_match_the_original() {
    // 2026-11-13 18:23:07 UTC, as the server wrote it (the sandbox runs in UTC).
    let t = Time1900::from_unix_ms(1_794_594_187_000);
    assert_eq!(format_history_time(t), "26/11/13 06:23:07 PM");
    assert_eq!(history_file_name(t), "KDX 26-11-13 06-23-07 PM.txt");
    // Midnight and noon are 12 o'clock.
    assert_eq!(
        format_history_time(Time1900::from_unix_ms(951_782_400_000)),
        "00/02/29 12:00:00 AM"
    );
    assert_eq!(
        format_history_time(Time1900::from_unix_ms(951_782_400_000 + 12 * 3_600_000)),
        "00/02/29 12:00:00 PM"
    );
}

#[test]
fn history_line_formats_like_the_server() {
    let line = HistoryLine::new(
        Time1900::from_unix_ms(1_794_594_187_000),
        b"Login",
        b"openkdx",
        b"admin",
        b"127.0.0.1:1",
    );
    assert_eq!(
        line.to_bytes(),
        b"26/11/13 06:23:07 PM\tLogin\topenkdx\tadmin\t127.0.0.1:1\t0\n"
    );
    assert!(parse_history(b"garbage without tabs\n").is_err());
    let f = parse_history(b"").unwrap();
    assert!(f.lines.is_empty() && !f.bom);
}

#[test]
fn legacy_dir_appends_history_and_starts_the_file_with_a_bom() {
    let tmp = tempfile::tempdir().unwrap();
    let mut store = LegacyDir::open(tmp.path()).unwrap();
    let t = Time1900::from_unix_ms(1_794_594_187_000);
    store.set_history_started(t);
    let line = HistoryLine::new(t, b"Login", b"n", b"l", b"d");
    let path = store.append_history(&line).unwrap();
    assert_eq!(
        path,
        tmp.path()
            .join("History")
            .join("KDX 26-11-13 06-23-07 PM.txt")
    );
    store.append_history(&line).unwrap();
    let bytes = fs::read(&path).unwrap();
    assert!(bytes.starts_with(b"\xef\xbb\xbf"));
    let f = parse_history(&bytes).unwrap();
    assert_eq!(f.lines.len(), 2);
    assert_eq!(
        f,
        HistoryFile {
            bom: true,
            lines: vec![line.clone(), line]
        }
    );
}

#[test]
fn news_archive_fixture_round_trips_and_matches_the_formatter() {
    let bytes = archive_fixture();
    let entries = parse_news_archive(&bytes).unwrap();
    assert_eq!(entries.len(), 4);
    assert_eq!(entries[0].subject, b"In second");
    assert_eq!(entries[0].poster, b"openkdx");
    assert_eq!(entries[0].time, b"Fri, 13 Nov 2026, 06:17:32 PM");
    assert_eq!(entries[0].body, b"Line one.\rLine two.");
    let mut again = b"\xef\xbb\xbf".to_vec();
    for e in &entries {
        again.extend(e.to_bytes());
    }
    assert_eq!(again, bytes);
    // The message still in News.dat before it was archived formats to the
    // same bytes.
    let s =
        decode_news(&fs::read(root().join("news/34-news-post-in-second.dat")).unwrap()).unwrap();
    let m = &s.groups[1].messages[0];
    assert_eq!(format_archive_entry(m), entries[0].to_bytes());
    assert_eq!(
        format_archive_time(Time1900::from_unix_ms(1_794_593_852_000)),
        "Fri, 13 Nov 2026, 06:17:32 PM"
    );
    assert_eq!(
        archive_file_name(b"Second Group", Time1900::from_unix_ms(1_794_593_852_000)),
        "Second Group 261113.txt"
    );
    let _: &ArchiveEntry = &entries[0];
}

#[test]
fn legacy_dir_appends_news_archive() {
    let tmp = tempfile::tempdir().unwrap();
    let store = LegacyDir::open(tmp.path()).unwrap();
    let s =
        decode_news(&fs::read(root().join("news/34-news-post-in-second.dat")).unwrap()).unwrap();
    let m = &s.groups[1].messages[0];
    let path = store.append_news_archive(b"Second Group", m).unwrap();
    assert_eq!(
        path,
        tmp.path()
            .join("News Archives/Second Group/Second Group 261113.txt")
    );
    store.append_news_archive(b"Second Group", m).unwrap();
    let bytes = fs::read(&path).unwrap();
    assert!(bytes.starts_with(b"\xef\xbb\xbf\n____"));
    assert_eq!(parse_news_archive(&bytes).unwrap().len(), 2);
}
