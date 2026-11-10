use openkdx_proto::registration::{Registration, RegistrationError, udp_crc, udp_xor};

fn sample() -> Registration {
    Registration {
        flags: 0,
        server_id: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12],
        server_port: 10700,
        users: 1,
        string0: vec![],
        login: b"trk-login".to_vec(),
        name: b"Lab Server".to_vec(),
        description: vec![],
        string4: vec![],
        groups: [
            b"Lab, Lab2".to_vec(),
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
        ],
        string11: vec![],
        string12: vec![],
        ignored_1c: [0; 8],
        ignored_26: [0; 13],
        padding: vec![],
    }
}

#[test]
fn udp_crc_matches_known_vector() {
    // CRC-32 poly 0x04c11db7 MSB-first, init 0xdeadbeef, no final XOR.
    // One byte 0x00 from init 0xdeadbeef: eight shifts of the register.
    let mut reg: u32 = 0xdead_beef;
    for _ in 0..8 {
        reg = if reg & 0x8000_0000 != 0 {
            (reg << 1) ^ 0x04c1_1db7
        } else {
            reg << 1
        };
    }
    assert_eq!(udp_crc(&[0]), reg);
    assert_eq!(udp_crc(&[]), 0xdead_beef);
}

#[test]
fn udp_xor_is_an_involution_and_uses_the_lcg() {
    let original: Vec<u8> = (0..12).collect();
    let mut data = original.clone();
    udp_xor(&mut data);
    let s0: u32 = 0xa5a1_6c4a;
    assert_eq!(&data[..4], &(0x0001_0203u32 ^ s0).to_be_bytes());
    let s1 = s0.wrapping_mul(0x41d2_8485).wrapping_add(0x322b);
    assert_eq!(&data[4..8], &(0x0405_0607u32 ^ s1).to_be_bytes());
    udp_xor(&mut data);
    assert_eq!(data, original);
}

#[test]
fn registration_round_trip_synthetic() {
    let reg = sample();
    let bytes = reg.encode();
    // 64-byte header + 9 + 10 + 9 = 92, already a multiple of 4
    assert_eq!(bytes.len(), 92);
    let decoded = Registration::decode(&bytes).unwrap();
    assert_eq!(decoded, reg);
    assert_eq!(decoded.encode(), bytes);
}

#[test]
fn registration_keeps_uncleared_padding() {
    let reg = Registration {
        name: b"Lab Serve".to_vec(), // 91 bytes of content → 1 byte of padding
        padding: vec![0x12],
        ..sample()
    };
    let bytes = reg.encode();
    assert_eq!(bytes.len(), 92);
    assert_eq!(Registration::decode(&bytes).unwrap(), reg);
}

#[test]
fn registration_rejects_bad_checksum_and_short_datagram() {
    let mut bytes = sample().encode();
    bytes[70] ^= 0xff;
    assert!(matches!(
        Registration::decode(&bytes),
        Err(RegistrationError::BadChecksum { .. })
    ));
    assert_eq!(
        Registration::decode(&bytes[..0x40]),
        Err(RegistrationError::TooShort)
    );
}

#[test]
fn registration_rejects_wrong_protocol_or_version() {
    let good = sample().encode();
    let mut plain = good.clone();
    udp_xor(&mut plain);
    plain[0] = b'X';
    let mut bad = plain.clone();
    udp_xor(&mut bad);
    assert_eq!(
        Registration::decode(&bad),
        Err(RegistrationError::BadProtocol)
    );
}
