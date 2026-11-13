use std::fs;
use std::path::{Path, PathBuf};

use openkdx_legacy::accounts::{AccountsFile, decode_accounts, encode_accounts};
use openkdx_legacy::{LegacyDir, LegacyError};
use openkdx_proto::AccountKey;
use openkdx_proto::msg::accounts::{Account, Privilege};

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/legacy/lab-2026-11-13/server/accounts")
}

fn fixture(name: &str) -> Vec<u8> {
    fs::read(dir().join(name)).unwrap()
}

fn all() -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<_> = fs::read_dir(dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "dat"))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                fs::read(&p).unwrap(),
            )
        })
        .collect();
    v.sort();
    v.push((
        "lab-2026-11-09".into(),
        fs::read(dir().join("../../../lab-2026-11-09/server/Accounts.dat")).unwrap(),
    ));
    assert_eq!(v.len(), 11);
    v
}

#[test]
fn every_accounts_fixture_round_trips() {
    for (name, bytes) in all() {
        let f = decode_accounts(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(encode_accounts(&f), bytes, "{name}");
    }
}

#[test]
fn fresh_accounts_has_admin_and_guest_and_12_classes() {
    let f = decode_accounts(&fixture("20-fresh.dat")).unwrap();
    assert_eq!(f.version, 2);
    assert_eq!(f.classes.len(), 12);
    assert_eq!(f.classes[0].name, b"Administrator");
    assert_eq!(f.classes[1].name, b"User");
    assert_eq!(f.classes[2].name, b"Guest");
    assert_eq!(f.classes[11].name, b"Custom Class 9");
    assert_eq!(f.accounts.len(), 2);
    assert_eq!(f.accounts[0].login, b"admin");
    assert_eq!(f.accounts[0].name, b"Administrator");
    assert_eq!(
        f.accounts[0].key,
        *AccountKey::from_password(b"openkdx-lab").as_bytes()
    );
    assert_eq!(f.accounts[1].login, b"guest");
    assert_eq!(f.accounts[1].key, *AccountKey::GUEST.as_bytes());
    assert_eq!(f.folder_access.len(), 3);
    assert_eq!(f.folder_access[0].name, b"[Default]");
    assert_eq!(f.folder_access[1].name, b"DB");
    assert_eq!(f.folder_access[2].name, b"UL");
    assert_eq!(f.find(b"admin").unwrap().class_index, 0);
    assert!(f.find(b"nobody").is_none());
}

#[test]
fn account_21_u1_has_class_1_and_key_of_p1() {
    let f = decode_accounts(&fixture("21-u1-created.dat")).unwrap();
    let u1 = f.find(b"u1").expect("u1");
    assert_eq!(u1.class_index, 1);
    assert_eq!(u1.name, b"Name X");
    assert_eq!(u1.key, *AccountKey::from_password(b"p1").as_bytes());
    assert_eq!(u1.editor, b"admin");
    let f = decode_accounts(&fixture("24-u1-password.dat")).unwrap();
    assert_eq!(
        f.find(b"u1").unwrap().key,
        *AccountKey::from_password(b"p2").as_bytes()
    );
    let f = decode_accounts(&fixture("26-class3-renamed.dat")).unwrap();
    assert_eq!(f.classes[3].name, b"Custom Renamed");
    let f = decode_accounts(&fixture("27-fa-pics.dat")).unwrap();
    assert_eq!(
        f.folder_access
            .iter()
            .map(|x| x.name.clone())
            .collect::<Vec<_>>(),
        [&b"[Default]"[..], b"DB", b"Pics", b"UL"]
    );
    let f = decode_accounts(&fixture("28-u1-deleted.dat")).unwrap();
    assert!(f.find(b"u1").is_none());
    assert!(f.find(b"u2").is_some());
}

#[test]
fn account_with_31_byte_login_round_trips() {
    let mut f = decode_accounts(&fixture("20-fresh.dat")).unwrap();
    let mut a = f.accounts[1].clone();
    a.login = vec![b'l'; 31];
    a.name = vec![0xa5; 7];
    a.key = *AccountKey::from_password(b"x").as_bytes();
    f.upsert(a.clone());
    assert_eq!(f.accounts.len(), 3);
    let bytes = encode_accounts(&f);
    let again = decode_accounts(&bytes).unwrap();
    assert_eq!(again.find(&[b'l'; 31]).unwrap().name, vec![0xa5; 7]);
    assert_eq!(encode_accounts(&again), bytes);
}

#[test]
fn upsert_then_remove_restores_the_fresh_bytes() {
    let fresh = fixture("20-fresh.dat");
    let mut f = decode_accounts(&fresh).unwrap();
    f.upsert(Account {
        login: b"tmp".to_vec(),
        ..Default::default()
    });
    assert_ne!(encode_accounts(&f), fresh);
    assert!(f.remove(b"tmp"));
    assert!(!f.remove(b"tmp"));
    assert_eq!(encode_accounts(&f), fresh);
    // Replacing keeps the account's position.
    let mut g = f.find(b"guest").unwrap().clone();
    g.name = b"Visitor".to_vec();
    f.upsert(g);
    assert_eq!(f.accounts[1].name, b"Visitor");
    assert_eq!(f.accounts.len(), 2);
}

#[test]
fn adding_an_account_lays_out_like_the_server() {
    // 20 → 21: the server added u1 after guest. Build the same record from
    // the decoded 21 and check the bytes match exactly, section table and
    // all (the 4-byte trailer differs between writes and is copied).
    let want = fixture("21-u1-created.dat");
    let target = decode_accounts(&want).unwrap();
    let mut f = decode_accounts(&fixture("20-fresh.dat")).unwrap();
    // The server also bumps counters inside admin's ignored bytes on every
    // write; take its records for the two existing accounts as they are.
    for a in &target.accounts {
        f.upsert(a.clone());
    }
    f.trailer = target.trailer.clone();
    assert_eq!(f.accounts.len(), 3);
    assert_eq!(encode_accounts(&f), want);
}

#[test]
fn accounts_errors() {
    assert!(matches!(
        decode_accounts(&[0; 10]),
        Err(LegacyError::BadMagic { .. })
    ));
    let mut b = fixture("20-fresh.dat");
    b.truncate(0x100);
    assert!(decode_accounts(&b).is_err());
    let f = AccountsFile::default();
    assert!(f.classes.is_empty());
}

#[test]
fn privileges_come_through() {
    let f = decode_accounts(&fixture("20-fresh.dat")).unwrap();
    assert!(f.classes[0].privileges.has(Privilege::ShutdownServer));
    assert!(!f.classes[2].privileges.has(Privilege::ShutdownServer));
    assert_eq!(
        f.folder_access[2].icon,
        openkdx_proto::msg::misc::FolderAccess::ICON_UPLOADS
    );
}

#[test]
fn legacy_dir_loads_and_saves_accounts() {
    let tmp = tempfile::tempdir().unwrap();
    fs::write(tmp.path().join("Accounts.dat"), fixture("20-fresh.dat")).unwrap();
    let store = LegacyDir::open(tmp.path()).unwrap();
    let mut f = store.load_accounts().unwrap().unwrap();
    let mut a = f.accounts[0].clone();
    a.login = b"second".to_vec();
    f.upsert(a);
    store.save_accounts(&f).unwrap();
    assert_eq!(store.load_accounts().unwrap().unwrap().accounts.len(), 3);
    assert_ne!(
        fs::read(tmp.path().join("Accounts.dat")).unwrap()[..8],
        b"%KDXAKNT"[..],
        "scrambled on disk"
    );
}
