use openkdx_legacy::scramble::{Stream, scramble, unscramble};
use openkdx_proto::AccountKey;

fn fixture(rel: &str) -> Vec<u8> {
    std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/legacy/").to_string() + rel)
        .unwrap()
}

#[test]
fn unscramble_known_plaintext_finds_admin_key() {
    let plain = unscramble(
        &fixture("lab-2026-11-13/server/accounts/20-fresh.dat"),
        Stream::SERVER_ACCOUNTS,
    );
    assert_eq!(&plain[..8], b"%KDXAKNT");
    let key = AccountKey::from_password(b"openkdx-lab");
    assert!(plain.windows(32).any(|w| w == key.as_bytes()), "admin key");
    assert!(plain.windows(5).any(|w| w == b"admin"));
    assert!(
        plain.windows(32).any(|w| w == AccountKey::GUEST.as_bytes()),
        "guest key"
    );
}

#[test]
fn unscramble_client_and_tracker_headers() {
    let plain = unscramble(
        &fixture("lab-2026-11-09/client/KDX.stg"),
        Stream::CLIENT_SETTINGS,
    );
    assert_eq!(&plain[..10], b"%KDXSTNG\x00\x02");
    assert!(plain.windows(7).any(|w| w == b"labuser"));
    let plain = unscramble(
        &fixture("lab-2026-11-13/tracker/KDXTrackerServ.stg"),
        Stream::TRACKER_SETTINGS,
    );
    assert_eq!(&plain[..10], b"%KDXTSTG\x00\x01");
    assert!(plain.windows(15).any(|w| w == b"New TrackerServ"));
}

#[test]
fn scramble_then_unscramble_is_identity() {
    for stream in [
        Stream::SERVER_ACCOUNTS,
        Stream::TRACKER_SETTINGS,
        Stream::CLIENT_SETTINGS,
    ] {
        for len in [0usize, 1, 3, 4, 5, 16, 2008, 661] {
            let plain: Vec<u8> = (0..len).map(|i| (i * 7 % 251) as u8).collect();
            let c = scramble(&plain, stream);
            assert_eq!(c.len(), len);
            assert_eq!(unscramble(&c, stream), plain);
            if len >= 4 {
                assert_ne!(c, plain);
            }
        }
    }
}

#[test]
fn scramble_is_deterministic_and_matches_the_original() {
    let c = fixture("lab-2026-11-13/server/accounts/20-fresh.dat");
    let plain = unscramble(&c, Stream::SERVER_ACCOUNTS);
    assert_eq!(scramble(&plain, Stream::SERVER_ACCOUNTS), c);
}

#[test]
fn keystream_is_the_documented_lcg() {
    // The first two words of the server stream, from docs/formats/accounts-dat.md.
    let ks = scramble(&[0; 8], Stream::SERVER_ACCOUNTS);
    assert_eq!(ks, [0x9a, 0xd2, 0x28, 0x61, 0xcf, 0xce, 0xef, 0x86]);
}
