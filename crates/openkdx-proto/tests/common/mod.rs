//! Shared helpers for the per-page message tests.
#![allow(dead_code)]

use openkdx_proto::msg::common::{Color, UserRecord};
use openkdx_proto::msg::{Message, Payload};

/// Encodes, decodes, and checks both the value and the bytes survive, through
/// the type and through `Message`.
pub fn round_trip<T: Payload + PartialEq + Clone>(value: T) -> Vec<u8> {
    let bytes = value.to_bytes();
    let back = T::decode(&bytes).unwrap_or_else(|e| panic!("{}: decode failed: {e}", T::KIND));
    assert!(back == value, "{}: value changed", T::KIND);
    assert_eq!(back.to_bytes(), bytes, "{}: bytes changed", T::KIND);
    let m = Message::decode(T::KIND, &bytes).unwrap();
    assert!(
        !matches!(m, Message::Unknown { .. }),
        "{}: not dispatched",
        T::KIND
    );
    assert_eq!(m.kind(), T::KIND);
    assert_eq!(m.encode(), bytes);
    bytes
}

/// Decoding must reject four trailing bytes (one could pass as `pad4`).
pub fn rejects_trailing<T: Payload>(bytes: &[u8]) {
    let mut longer = bytes.to_vec();
    longer.extend_from_slice(&[0xee; 4]);
    assert!(
        T::decode(&longer).is_err(),
        "{}: accepted trailing bytes",
        T::KIND
    );
}

pub fn user(id: u32, nick: &[u8], padding: &[u8]) -> UserRecord {
    UserRecord {
        user_id: id,
        status: 0,
        ignored_05: [0; 5],
        client_port: 10600,
        text_color: Color::default(),
        background_color: Color {
            r: 0xff,
            g: 0xcc,
            b: 0,
        },
        account_color: Color::default(),
        flags: 0,
        ignored_1b: 0,
        icon: vec![0x11; 8],
        extra: vec![],
        nick: nick.to_vec(),
        info: b"info".to_vec(),
        padding: padding.to_vec(),
    }
}
