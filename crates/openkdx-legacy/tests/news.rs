use std::fs;
use std::path::{Path, PathBuf};

use openkdx_legacy::news::{NewsGroupRecord, NewsStore, decode_news, encode_news};
use openkdx_legacy::{LegacyDir, LegacyError};
use openkdx_proto::msg::news::{NewsMessage, NewsgroupInfo};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/legacy/lab-2026-11-13/server/news")
}

fn fixture(name: &str) -> Vec<u8> {
    fs::read(dir().join(name)).unwrap()
}

#[test]
fn every_news_fixture_round_trips() {
    let mut names: Vec<_> = fs::read_dir(dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    names.sort();
    assert_eq!(names.len(), 7);
    for path in names {
        let bytes = fs::read(&path).unwrap();
        let s = decode_news(&bytes).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        assert_eq!(encode_news(&s), bytes, "{path:?}");
    }
}

#[test]
fn empty_news_dat_is_empty_store() {
    let s = decode_news(&[]).unwrap();
    assert!(s.groups.is_empty());
    assert_eq!(encode_news(&s), b"");
    assert_eq!(NewsStore::default(), s);
}

#[test]
fn news_31_has_one_empty_group() {
    let s = decode_news(&fixture("31-news-empty.dat")).unwrap();
    assert_eq!(s.version, 2);
    assert_eq!(s.groups.len(), 1);
    let g = &s.groups[0];
    assert_eq!(g.info.group_id, 1);
    assert_eq!(g.info.name, b"General News");
    assert_eq!(g.info.size_limit_kib, 50);
    assert_eq!(g.info.message_count, 0);
    assert_eq!(g.last_message_id, 1);
    assert!(g.messages.is_empty());
}

#[test]
fn news_32_has_one_post_with_subject_first() {
    let s = decode_news(&fixture("32-news-one-post.dat")).unwrap();
    let g = &s.groups[0];
    assert_eq!(g.info.message_count, 1);
    assert_eq!(g.last_message_id, 2);
    assert_eq!(g.messages.len(), 1);
    let m = &g.messages[0];
    assert_eq!(m.message_id, 2, "a fresh group's first post is 2");
    assert_eq!(m.group_id, 1);
    assert_eq!(m.poster, b"openkdx");
    assert_eq!(m.login, b"admin");
    assert_eq!(m.subject, b"First");
    assert_eq!(m.body, b"Hello from the lab.");
}

#[test]
fn news_33_second_group_size_8_and_34_two_line_body() {
    let s = decode_news(&fixture("33-news-two-groups.dat")).unwrap();
    assert_eq!(s.groups.len(), 2);
    assert_eq!(s.groups[1].info.name, b"Second Group");
    assert_eq!(s.groups[1].info.description, b"Another group");
    assert_eq!(s.groups[1].info.size_limit_kib, 8);
    assert_eq!(s.groups[1].info.group_id, 2);
    let s = decode_news(&fixture("34-news-post-in-second.dat")).unwrap();
    assert_eq!(s.groups[1].messages[0].body, b"Line one.\rLine two.");
    let s = decode_news(&fixture("35-news-deleted.dat")).unwrap();
    assert!(s.groups[0].messages.is_empty(), "deleted, not tombstoned");
    assert_eq!(s.groups[0].last_message_id, 2, "ids are not reused");
    let s = decode_news(&fixture("36-news-size-4.dat")).unwrap();
    assert_eq!(s.groups[1].info.size_limit_kib, 4);
    let s = decode_news(&fixture("37-news-archived.dat")).unwrap();
    assert_eq!(s.groups[1].messages.len(), 1, "older posts archived away");
    assert_eq!(s.groups[1].last_message_id, 6);
    assert_eq!(s.groups[1].messages[0].subject, b"Big 4");
}

#[test]
fn post_appended_by_us_decodes_and_lays_out_like_the_server() {
    // 31 → 32: the server appended one post. Build it ourselves.
    let want = fixture("32-news-one-post.dat");
    let target = decode_news(&want).unwrap();
    let mut s = decode_news(&fixture("31-news-empty.dat")).unwrap();
    let mut m = target.groups[0].messages[0].clone();
    assert_eq!(s.next_message_id(1), Some(2));
    m.message_id = 0;
    assert_eq!(s.post(1, m.clone()).unwrap(), 2);
    assert_eq!(s.groups[0].last_message_id, 2);
    assert_eq!(s.groups[0].info.message_count, 1);
    // The group's modified time moves too, and the gaps hold whatever the
    // server's buffer had; take the server's.
    s.groups[0].info.time = target.groups[0].info.time;
    s.groups_gap = target.groups_gap.clone();
    s.trailer = target.trailer.clone();
    assert_eq!(encode_news(&s), want);
    assert!(s.post(9, m).is_err());
}

#[test]
fn create_group_and_delete_message_api() {
    let mut s = NewsStore::default();
    let id = s.create_group(NewsgroupInfo {
        name: b"Fresh".to_vec(),
        size_limit_kib: 50,
        read_classes: u32::MAX,
        post_classes: u32::MAX,
        ..Default::default()
    });
    assert_eq!(id, 1);
    assert_eq!(s.groups[0].info.group_id, 1);
    s.post(
        1,
        NewsMessage {
            subject: b"a".to_vec(),
            ..Default::default()
        },
    )
    .unwrap();
    s.post(
        1,
        NewsMessage {
            subject: b"b".to_vec(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        s.groups[0]
            .messages
            .iter()
            .map(|m| m.message_id)
            .collect::<Vec<_>>(),
        [2, 3]
    );
    assert!(s.delete_message(1, 2));
    assert!(!s.delete_message(1, 2));
    assert_eq!(s.groups[0].info.message_count, 1);
    let bytes = encode_news(&s);
    assert_eq!(bytes.len() % 8, 0, "the file ends 8-aligned");
    let again = decode_news(&bytes).unwrap();
    assert_eq!(again.groups[0].messages[0].subject, b"b");
    assert_eq!(again.groups[0].last_message_id, 3);
    assert_eq!(encode_news(&again), bytes);
    assert!(s.delete_group(1));
    assert!(s.groups.is_empty());
    let _: &NewsGroupRecord;
}

#[test]
fn news_errors() {
    let mut b = fixture("32-news-one-post.dat");
    b[5] = b'X';
    assert!(matches!(decode_news(&b), Err(LegacyError::BadMagic { .. })));
    let mut b = fixture("32-news-one-post.dat");
    b[9] = 3;
    assert!(matches!(
        decode_news(&b),
        Err(LegacyError::UnsupportedVersion { .. })
    ));
    assert!(decode_news(&fixture("32-news-one-post.dat")[..0x90]).is_err());
}

#[test]
fn legacy_dir_loads_and_saves_news() {
    let tmp = tempfile::tempdir().unwrap();
    let store = LegacyDir::open(tmp.path()).unwrap();
    assert!(store.load_news().unwrap().is_none());
    fs::write(tmp.path().join("News.dat"), b"").unwrap();
    let mut s = store.load_news().unwrap().unwrap();
    assert!(s.groups.is_empty());
    s.create_group(NewsgroupInfo {
        name: b"G".to_vec(),
        ..Default::default()
    });
    store.save_news(&s).unwrap();
    assert_eq!(store.load_news().unwrap().unwrap().groups.len(), 1);
}
