//! Messages 4000–4201 (gconsole.md) and the `%HGC` handshake rules.

mod common;

use common::{rejects_trailing, round_trip};
use openkdx_proto::ChallengeTable;
use openkdx_proto::handshake::{HandshakeError, Hello, Role, SubProtocol, table_for_peer};
use openkdx_proto::msg::Payload;
use openkdx_proto::msg::gconsole::*;
use openkdx_proto::session::{Header, Scramble};

#[test]
fn open_window() {
    let bytes = round_trip(OpenWindow {
        width: 0x200,
        height: 0x180,
        color: 0x0033_6699,
        rest: [0; 16],
    });
    assert_eq!(bytes.len(), 24);
    assert_eq!(&bytes[..4], &[2, 0, 1, 0x80]);
    rejects_trailing::<OpenWindow>(&bytes);
}

#[test]
fn draw_records_carry_kind_and_length_in_one_word() {
    let create = DrawRecord {
        kind: 2,
        body: vec![0; 0x14],
        padding: vec![],
    };
    let pixel = DrawRecord {
        kind: 103,
        body: vec![0, 1, 0, 5, 0, 6, 0, 0xff, 0, 0],
        padding: vec![0xaa, 0xbb],
    };
    let bytes = round_trip(Draw {
        records: vec![create.clone(), pixel.clone()],
    });
    // record 1: 4 + 0x14 = 24; record 2: 4 + 10 + 2 = 16
    assert_eq!(bytes.len(), 40);
    let word = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    assert_eq!(word >> 22, 2);
    assert_eq!(word & 0x3f_ffff, 0x14);
    let word = u32::from_be_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
    assert_eq!(word >> 22, 103);
    assert_eq!(word & 0x3f_ffff, 10);
    assert_eq!(&bytes[38..], &[0xaa, 0xbb]);
    assert_eq!(Draw::decode(&[]).unwrap().records.len(), 0);
    assert!(Draw::decode(&bytes[..30]).is_err(), "a cut body");
    assert_eq!(pixel.object_id(), Some(1));
    assert_eq!(DrawRecord::KIND_CREATE, 2);
}

#[test]
fn key_and_mouse() {
    let bytes = round_trip(Key {
        time: 1000,
        modifiers: 8,
        character: 0x41,
        usage: 4,
        kind: 1,
        value_0f: 0,
    });
    assert_eq!(bytes.len(), 16);
    let bytes = round_trip(Mouse {
        time: 1000,
        value_04: 0,
        x: 10,
        y: 20,
        button: 4,
        kind: 10,
        wheel: -0x78,
    });
    assert_eq!(bytes.len(), 16);
    assert_eq!(&bytes[14..], &[0xff, 0x88]);
}

fn hello(sp: SubProtocol, sv: u8) -> Hello {
    Hello {
        sub_protocol: sp,
        version: 1,
        sub_version: sv,
        challenge: 2832,
        nonce: [0; 12],
        name: vec![],
        pad_len: 0,
        skipped_len: 0,
        pad: Vec::new(),
        skipped: Vec::new(),
        ignored: [0; 4],
        padding: vec![],
    }
}

#[test]
fn hgc_hello_round_trip() {
    let h = hello(SubProtocol::Hgc, 0x40);
    let bytes = h.encode(5);
    let (back, used) = Hello::decode(&bytes).unwrap();
    assert_eq!(used, 36);
    assert_eq!(back, h);
    assert_eq!(back.role(), Some(Role::GConsole));
}

#[test]
fn challenge_table_switch_rule() {
    let kdx = ChallengeTable::new(vec![1; 1739]);
    let hgc = ChallengeTable::new(vec![2; 3193]);
    let peer = hello(SubProtocol::Hgc, 0x40);
    assert_eq!(table_for_peer(&peer, &kdx, Some(&hgc)).unwrap(), &hgc);
    assert_eq!(
        table_for_peer(&peer, &kdx, None),
        Err(HandshakeError::MissingGConsoleTable)
    );
    // the peer's challenge 2832 is past the KDX story's end, within the GConsole story
    assert!(kdx.response(2832).is_none());
    assert!(hgc.response(2832).is_some());
    let server = hello(SubProtocol::Kdx, 0x20);
    assert_eq!(table_for_peer(&server, &kdx, Some(&hgc)).unwrap(), &kdx);
    assert_eq!(
        table_for_peer(&hello(SubProtocol::Hgc, 0x41), &kdx, Some(&hgc)),
        Err(HandshakeError::TooNew)
    );
    assert_eq!(
        table_for_peer(&hello(SubProtocol::Hgc, 0x3f), &kdx, Some(&hgc)),
        Err(HandshakeError::TooOld)
    );
}

#[test]
fn hgc_scramble_matches_the_table_and_inverts() {
    let mut h = [0u8; 16];
    h[3] = 1;
    h[7] = 2;
    h[11] = 3;
    h[15] = 4;
    let original = h;
    Scramble::Hgc.apply(&mut h);
    let w = |i: usize| u32::from_be_bytes([h[i], h[i + 1], h[i + 2], h[i + 3]]);
    assert_eq!(w(0), 2u32.rotate_left(15) ^ 0x5f54_7a17);
    assert_eq!(w(4), 3u32.rotate_right(4) ^ 0x69c8_3e35);
    assert_eq!(w(8), 1u32.rotate_left(7) ^ 0x1b20_e200);
    assert_eq!(w(12), 4u32.rotate_right(5) ^ 0x8022_e8d1);
    Scramble::Hgc.reverse(&mut h);
    assert_eq!(h, original);

    let mut k = original;
    Scramble::Kdx.apply(&mut k);
    openkdx_proto::session::unscramble(&mut k);
    assert_eq!(k, original);
    assert_eq!(Scramble::for_role(Role::GConsole), Scramble::Hgc);
    assert_eq!(Scramble::for_role(Role::Server), Scramble::Kdx);

    let cipher = openkdx_proto::session::SessionCipher::new(&openkdx_proto::AccountKey::GUEST)
        .with_scramble(Scramble::Hgc);
    let header = Header::request(9, 4000, 24);
    let block = cipher.seal_header(header);
    assert_eq!(cipher.open_header(&block, 0x100000).unwrap(), header);
    let kdx_cipher = openkdx_proto::session::SessionCipher::new(&openkdx_proto::AccountKey::GUEST);
    assert!(kdx_cipher.open_header(&block, 0x100000) != Ok(header));
}
