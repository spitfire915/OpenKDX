use openkdx_proto::AccountKey;
use openkdx_proto::session::{
    Header, SERVER_RECV_CAP, SessionCipher, SessionError, padding_word, scramble, unscramble,
};

fn words(h: &[u8; 16]) -> [u32; 4] {
    let (w, _) = h.as_chunks::<4>();
    [
        u32::from_be_bytes(w[0]),
        u32::from_be_bytes(w[1]),
        u32::from_be_bytes(w[2]),
        u32::from_be_bytes(w[3]),
    ]
}

#[test]
fn scramble_then_unscramble_is_identity() {
    let original: [u8; 16] = [
        0x6f, 0x2c, 0x62, 0xe0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0x02, 0x2e,
    ];
    let mut h = original;
    scramble(&mut h);
    assert_ne!(h, original);
    unscramble(&mut h);
    assert_eq!(h, original);
}

#[test]
fn scramble_matches_formula_on_known_words() {
    let mut h = [0u8; 16];
    h[3] = 1;
    h[7] = 2;
    h[11] = 3;
    h[15] = 4;
    scramble(&mut h);
    let w = words(&h);
    assert_eq!(w[0], 2u32.rotate_left(13) ^ 0x6e7d_fd34);
    assert_eq!(w[1], 3u32.rotate_right(6) ^ 0xe152_da04);
    assert_eq!(w[2], 1u32.rotate_left(7) ^ 0xa95a_759b);
    assert_eq!(w[3], 4u32.rotate_right(3) ^ 0x0699_2e25);
}

#[test]
fn header_round_trip() {
    let h = Header::request(0x6f2c_62e0, 100, 558);
    assert!(!h.reply);
    let bytes = h.to_bytes();
    assert_eq!(&bytes[..4], &[0x6f, 0x2c, 0x62, 0xe0]);
    assert_eq!(&bytes[4..8], &[0, 0, 0, 100]);
    assert_eq!(bytes[8], 0);
    assert_eq!(bytes[9], 0);
    assert_eq!(&bytes[10..], &[0, 0, 0, 0, 0x02, 0x2e]);
    assert_eq!(Header::from_bytes(&bytes, SERVER_RECV_CAP).unwrap(), h);

    let r = Header::reply(7, 101, 560);
    assert!(r.reply);
    assert_eq!(r.to_bytes()[9], 1);
    assert_eq!(
        Header::from_bytes(&r.to_bytes(), SERVER_RECV_CAP).unwrap(),
        r
    );
    assert_eq!(r.padded_len(), 560);
    assert_eq!(Header::request(1, 1, 1).padded_len(), 16);
    assert_eq!(Header::request(1, 1, 0).padded_len(), 0);
}

#[test]
fn header_rejects_zero_txid_and_kind() {
    let zero_txid = Header::request(0, 100, 0).to_bytes();
    assert_eq!(
        Header::from_bytes(&zero_txid, SERVER_RECV_CAP),
        Err(SessionError::ZeroTxidOrKind)
    );
    let zero_kind = Header::request(5, 0, 0).to_bytes();
    assert_eq!(
        Header::from_bytes(&zero_kind, SERVER_RECV_CAP),
        Err(SessionError::ZeroTxidOrKind)
    );
    assert_eq!(
        Header::from_bytes_unbounded(&zero_kind),
        Err(SessionError::ZeroTxidOrKind)
    );
}

#[test]
fn header_rejects_len_over_cap_and_high_bits() {
    let big = Header::request(5, 414, 0x20001).to_bytes();
    assert_eq!(
        Header::from_bytes(&big, 0x20000),
        Err(SessionError::LengthTooLong {
            len: 0x20001,
            cap: 0x20000
        })
    );
    let huge = Header::request(5, 414, 1 << 32).to_bytes();
    assert!(matches!(
        Header::from_bytes(&huge, u64::MAX),
        Err(SessionError::LengthTooLong { .. })
    ));
    assert_eq!(Header::from_bytes_unbounded(&huge).unwrap().len, 1 << 32);
}

#[test]
fn header_reserved_byte_must_be_zero() {
    let mut b = Header::request(5, 100, 0).to_bytes();
    b[8] = 1;
    assert_eq!(
        Header::from_bytes(&b, SERVER_RECV_CAP),
        Err(SessionError::ReservedNotZero)
    );
}

#[test]
fn padding_word_is_the_lcg() {
    assert_eq!(
        padding_word(7),
        7u32.wrapping_mul(0x41c6_4e6d).wrapping_add(0x3039)
    );
}

#[test]
fn seal_pads_last_block_with_txid_lcg() {
    let cipher = SessionCipher::new(&AccountKey::GUEST);
    let header = Header::request(7, 207, 5);
    let sealed = cipher.seal(header, b"hello");
    assert_eq!(sealed.len(), 32);
    let mut block = sealed[16..].to_vec();
    cipher.decrypt_blocks(&mut block);
    assert_eq!(&block[..5], b"hello");
    // little-endian memory words from the payload start [O first-connect 10, 13]:
    // bytes 5..8 are the tail of word 1, then words 2 and 3
    let pad = padding_word(7).to_le_bytes();
    assert_eq!(&block[5..8], &pad[1..]);
    assert_eq!(&block[8..12], &pad);
    assert_eq!(&block[12..16], &pad);
}

#[test]
fn seal_open_round_trip_with_lab_key() {
    let cipher = SessionCipher::new(&AccountKey::from_password(b"openkdx-lab"));
    let header = Header::reply(0xc3b7_1d01, 101, 33);
    let payload: Vec<u8> = (0..33u8).collect();
    let sealed = cipher.seal(header, &payload);
    assert_eq!(sealed.len(), 16 + 48);
    let first: [u8; 16] = sealed[..16].try_into().unwrap();
    let got = cipher.open_header(&first, SERVER_RECV_CAP).unwrap();
    assert_eq!(got, header);
    assert_eq!(cipher.open_payload(&got, &sealed[16..]).unwrap(), payload);
    assert_eq!(cipher.seal_header(header), first);
}

#[test]
fn open_payload_rejects_wrong_block_count() {
    let cipher = SessionCipher::new(&AccountKey::GUEST);
    let header = Header::request(1, 100, 20);
    assert_eq!(
        cipher.open_payload(&header, &[0u8; 16]),
        Err(SessionError::BadBlockLength)
    );
    assert_eq!(
        cipher.open_payload(&header, &[0u8; 48]),
        Err(SessionError::BadBlockLength)
    );
}

#[test]
fn wrong_key_does_not_open_a_header() {
    let a = SessionCipher::new(&AccountKey::GUEST);
    let b = SessionCipher::new(&AccountKey::from_password(b"openkdx-lab"));
    let block = a.seal_header(Header::request(9, 100, 0));
    // With the wrong key the header is noise; the checks almost surely fail.
    assert!(b.open_header(&block, SERVER_RECV_CAP).is_err());
}
