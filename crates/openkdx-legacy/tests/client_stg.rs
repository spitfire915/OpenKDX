use std::fs;
use std::path::{Path, PathBuf};

use openkdx_legacy::LegacyError;
use openkdx_legacy::client_stg::{
    AddressBookEntry, ClientSettings, decode_client_stg, encode_client_stg,
};
use openkdx_proto::AccountKey;

fn dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/legacy/lab-2026-11-13/client")
}

fn fixture(name: &str) -> Vec<u8> {
    fs::read(dir().join(name)).unwrap()
}

fn all() -> Vec<(String, Vec<u8>)> {
    let mut v: Vec<_> = fs::read_dir(dir())
        .unwrap()
        .map(|e| e.unwrap().path())
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                fs::read(&p).unwrap(),
            )
        })
        .collect();
    v.push((
        "lab-2026-11-09".into(),
        fs::read(dir().join("../../lab-2026-11-09/client/KDX.stg")).unwrap(),
    ));
    v.sort();
    assert_eq!(v.len(), 6);
    v
}

#[test]
fn client_stg_fixtures_round_trip() {
    for (name, bytes) in all() {
        let s = decode_client_stg(&bytes).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(encode_client_stg(&s), bytes, "{name}");
    }
}

#[test]
fn client_fresh_identity_and_stock_address_book() {
    let s = decode_client_stg(&fixture("60-client-fresh.stg")).unwrap();
    assert_eq!(s.version, 2);
    assert_eq!(s.identity.name, b"labuser");
    assert_eq!(s.identity.description, b"");
    assert_eq!(
        s.identity.background_color,
        openkdx_proto::msg::common::Color {
            r: 0xff,
            g: 0xcc,
            b: 0
        }
    );
    assert_eq!(
        s.identity.text_color,
        openkdx_proto::msg::common::Color { r: 0, g: 0, b: 0 }
    );
    assert_eq!(s.identity.icon.len(), 512);
    assert_eq!(s.address_book.len(), 1);
    assert_eq!(s.address_book[0].name, b"Official Haxial Server");
    assert_eq!(s.address_book[0].address, b"kdxserver.haxial.com");
    assert_eq!(s.address_book[0].key, *AccountKey::GUEST.as_bytes());
    assert_eq!(s.order, vec![0, 1, 7, 8, 10, 9, 11]);
}

#[test]
fn client_61_nick_62_description_63_64_address_book() {
    let s = decode_client_stg(&fixture("61-client-nick.stg")).unwrap();
    assert_eq!(s.identity.name, b"legacynick");
    let s = decode_client_stg(&fixture("62-client-description.stg")).unwrap();
    assert_eq!(s.identity.description, b"A client description");
    let s = decode_client_stg(&fixture("63-client-address-book.stg")).unwrap();
    assert_eq!(s.address_book.len(), 2);
    let lab = &s.address_book[1];
    assert_eq!(lab.name, b"Lab");
    assert_eq!(lab.address, b"127.0.0.1");
    assert_eq!(lab.login, b"admin");
    assert_eq!(lab.description, b"The lab server");
    assert_eq!(
        lab.key,
        *AccountKey::from_password(b"openkdx-lab").as_bytes(),
        "the password is stored as the derived key"
    );
    let s = decode_client_stg(&fixture("64-client-address-book-2.stg")).unwrap();
    assert_eq!(s.address_book.len(), 3);
    assert_eq!(s.address_book[2].name, b"Second");
    assert_eq!(s.address_book[2].address, b"10.0.0.2");
    assert_eq!(s.address_book[2].login, b"");
    assert_eq!(
        s.address_book.iter().map(|e| e.id).collect::<Vec<_>>(),
        [1, 2, 3]
    );
}

#[test]
fn set_nick_and_add_entry_then_round_trip() {
    let mut s = decode_client_stg(&fixture("60-client-fresh.stg")).unwrap();
    s.identity.name = b"openkdxwrote".to_vec();
    s.address_book.push(AddressBookEntry {
        id: 0,
        name: b"Mine".to_vec(),
        address: b"example.org".to_vec(),
        login: b"me".to_vec(),
        key: *AccountKey::from_password(b"pw").as_bytes(),
        ..s.address_book[0].clone()
    });
    let bytes = encode_client_stg(&s);
    let again = decode_client_stg(&bytes).unwrap();
    assert_eq!(again.identity.name, b"openkdxwrote");
    assert_eq!(again.address_book[1].name, b"Mine");
    assert_eq!(
        again.address_book[1].id, 2,
        "ids are assigned on encode when 0"
    );
    assert_eq!(encode_client_stg(&again), bytes);
    assert_ne!(bytes[..8], b"%KDXSTNG"[..], "scrambled on disk");
}

#[test]
fn adding_an_entry_lays_out_like_the_client() {
    let want = fixture("63-client-address-book.stg");
    let target = decode_client_stg(&want).unwrap();
    let mut s = decode_client_stg(&fixture("62-client-description.stg")).unwrap();
    s.address_book.push(target.address_book[1].clone());
    s.address_book_count2 = target.address_book_count2;
    // Window positions and the like move between runs; take section 0.
    s.sections = target.sections.clone();
    assert_eq!(encode_client_stg(&s), want);
}

#[test]
fn client_stg_errors() {
    assert!(matches!(
        decode_client_stg(&[0u8; 0x90]),
        Err(LegacyError::BadMagic { .. })
    ));
    assert!(decode_client_stg(&fixture("60-client-fresh.stg")[..0x100]).is_err());
    let _ = ClientSettings::default;
}

#[test]
fn address_book_header_word_and_ids_follow_the_entries() {
    let mut s = decode_client_stg(&fixture("64-client-address-book-2.stg")).unwrap();
    s.address_book.push(AddressBookEntry {
        id: 0,
        name: b"Fourth".to_vec(),
        ..Default::default()
    });
    let again = decode_client_stg(&encode_client_stg(&s)).unwrap();
    assert_eq!(again.address_book_count2, 4);
    assert_eq!(again.address_book[3].id, 4);
    // Deleting the middle entry keeps the others' ids and the word tracks the max.
    s.address_book.remove(1);
    let again = decode_client_stg(&encode_client_stg(&s)).unwrap();
    assert_eq!(
        again.address_book.iter().map(|e| e.id).collect::<Vec<_>>(),
        [1, 3, 4]
    );
    assert_eq!(again.address_book_count2, 4);
    // An id at the top of the range does not overflow.
    s.address_book[0].id = u32::MAX;
    s.address_book.push(AddressBookEntry {
        id: 0,
        ..Default::default()
    });
    let _ = encode_client_stg(&s);
}

#[test]
fn hostile_order_or_count_in_client_stg_is_an_error() {
    use openkdx_legacy::scramble::{Stream, scramble, unscramble};
    let mut plain = unscramble(&fixture("60-client-fresh.stg"), Stream::CLIENT_SETTINGS);
    // The address book's count word (section 7 at 0x4c8, +4).
    plain[0x4cc..0x4d0].copy_from_slice(&0xffff_ffffu32.to_be_bytes());
    assert!(decode_client_stg(&scramble(&plain, Stream::CLIENT_SETTINGS)).is_err());
    let mut s = decode_client_stg(&fixture("60-client-fresh.stg")).unwrap();
    s.order.push(40);
    let _ = encode_client_stg(&s);
}
