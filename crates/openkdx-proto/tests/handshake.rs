//! Handshake vectors from `captures/first-connect.pcapng`, `guest-login.pcapng`
//! and `tracker-basic.pcapng` (lab admin password `openkdx-lab`).

use openkdx_proto::AccountKey;
use openkdx_proto::challenge::ChallengeTable;
use openkdx_proto::handshake::{
    HandshakeError, Hello, Proof, ProofStatus, Role, SubProtocol, proof_auth,
};
use openkdx_proto::xor::xor_stream;

fn h(s: &str) -> Vec<u8> {
    hex::decode(s).unwrap()
}

fn nonce(s: &str) -> [u8; 12] {
    h(s).try_into().unwrap()
}

#[test]
fn account_key_of_lab_password() {
    assert_eq!(
        hex::encode(AccountKey::from_password(b"openkdx-lab").as_bytes()),
        "7720015b5397c4e1910d0edf6e98f7aec7cbdbed7720015b5397c4e177390992"
    );
}

#[test]
fn guest_key_is_the_fixed_constant() {
    assert_eq!(
        hex::encode(AccountKey::from_password(b"").as_bytes()),
        "da39a3eecaaebe6e5e6b4b0dba8e29ba3255bfef18a2b5c495601890afd80709"
    );
    assert_eq!(AccountKey::from_password(b""), AccountKey::GUEST);
}

#[test]
fn first_connect_client_proof_auth() {
    // frame 8: the client's proof uses the server's nonce (frame 6)
    let key = AccountKey::from_password(b"openkdx-lab");
    let auth = proof_auth(&key, &nonce("d51503a414c2987f0d832176"));
    assert_eq!(
        hex::encode(auth),
        "040f3e1528378f64484b05f0e31595c1861ca170"
    );
}

#[test]
fn first_connect_server_proof_auth() {
    // frame 9: the server's proof uses the client's nonce (frame 4)
    let key = AccountKey::from_password(b"openkdx-lab");
    let auth = proof_auth(&key, &nonce("49f403a414c2980df0150354"));
    assert_eq!(
        hex::encode(auth),
        "4d9fa57bbf71ff0f6a6c579a3cffcaacdfad97a9"
    );
}

#[test]
fn guest_login_proofs() {
    let client = proof_auth(&AccountKey::GUEST, &nonce("004203a414c343df30a44f86"));
    assert_eq!(
        hex::encode(client),
        "b92d57c73ef822232ee4a944367d55676b1bf90f"
    );
    let server = proof_auth(&AccountKey::GUEST, &nonce("58b203a414c343a9425e0150"));
    assert_eq!(
        hex::encode(server),
        "4e1e98512e3e97bd88ecb5f6aec01ff0379a5cb7"
    );
}

#[test]
fn tracker_proofs_use_guest_key() {
    let client = proof_auth(&AccountKey::GUEST, &nonce("9baa03a414c8a6a6043cb0e6"));
    assert_eq!(
        hex::encode(client),
        "1545f70da2502a2f3d8e22735c9cf5cd920c03d7"
    );
    let server = proof_auth(&AccountKey::GUEST, &nonce("33ab03a414c8a6d9c2c5bc7d"));
    assert_eq!(
        hex::encode(server),
        "cd8334dca3f8cfdde2b634469ffceee61fd5ccf0"
    );
}

#[test]
fn xor_stream_round_trips_and_matches_first_word() {
    let key = 0x1234_5678u32;
    let mut body = 0xdead_beefu32.to_be_bytes().to_vec();
    body.extend_from_slice(&[1, 2, 3, 4]);
    let original = body.clone();
    xor_stream(key, &mut body);
    let k1 = (key << 1).wrapping_add(0x4878);
    assert_eq!(&body[..4], &(0xdead_beefu32 ^ k1).to_be_bytes());
    let k2 = (k1 << 1).wrapping_add(0x4878);
    assert_eq!(&body[4..], &(0x0102_0304u32 ^ k2).to_be_bytes());
    xor_stream(key, &mut body);
    assert_eq!(body, original);
}

fn client_hello() -> Hello {
    Hello {
        sub_protocol: SubProtocol::Kdx,
        version: 1,
        sub_version: 0x10,
        challenge: 1037,
        nonce: nonce("49f403a414c2980df0150354"),
        name: b"admin".to_vec(),
        pad_len: 0,
        skipped_len: 0,
        ignored: [0; 4],
        padding: vec![0; 3],
    }
}

#[test]
fn hello_encode_decode_round_trip_first_connect_values() {
    let hello = client_hello();
    assert_eq!(hello.wire_len(), 44);
    let bytes = hello.encode(0xcafe_f00d);
    assert_eq!(bytes.len(), 44);
    assert_eq!(&bytes[..4], &0xcafe_f00du32.to_be_bytes());
    let (decoded, used) = Hello::decode(&bytes).unwrap();
    assert_eq!(used, 44);
    assert_eq!(decoded, hello);

    let server = Hello {
        sub_version: 0x20,
        challenge: 883,
        nonce: nonce("d51503a414c2987f0d832176"),
        name: vec![],
        padding: vec![],
        ..client_hello()
    };
    assert_eq!(server.wire_len(), 36);
    let bytes = server.encode(1);
    assert_eq!(Hello::decode(&bytes).unwrap(), (server, 36));
}

#[test]
fn hello_decode_wants_more_bytes_when_short() {
    let bytes = client_hello().encode(7);
    assert!(matches!(
        Hello::decode(&bytes[..10]),
        Err(HandshakeError::Incomplete { .. })
    ));
    assert_eq!(
        Hello::decode(&bytes[..40]),
        Err(HandshakeError::Incomplete { needed: 4 })
    );
}

#[test]
fn hello_skips_pad_and_truncates_name_to_31() {
    let hello = Hello {
        pad_len: 3,
        skipped_len: 2,
        name: vec![b'x'; 40],
        padding: vec![],
        ..client_hello()
    };
    // (40 + 3 + 2 + 0x27) & !3 = 84
    assert_eq!(hello.wire_len(), 84);
    let bytes = hello.encode(99);
    assert_eq!(bytes.len(), 84);
    let (decoded, used) = Hello::decode(&bytes).unwrap();
    assert_eq!(used, 84);
    assert_eq!(decoded.name, vec![b'x'; 31]);
    assert_eq!(decoded.pad_len, 3);
    assert_eq!(decoded.skipped_len, 2);
}

#[test]
fn hello_rejects_wrong_protocol_version_and_length() {
    let good = client_hello().encode(5);
    let mut bad = good.clone();
    // bytes 4..8 are "%TXP" under the stream; flip one bit of the ciphertext
    bad[7] ^= 1;
    assert_eq!(Hello::decode(&bad), Err(HandshakeError::UnknownProtocol));

    let old = Hello {
        version: 0,
        ..client_hello()
    }
    .encode(5);
    assert_eq!(Hello::decode(&old), Err(HandshakeError::TooOld));
    let new = Hello {
        version: 2,
        ..client_hello()
    }
    .encode(5);
    assert_eq!(Hello::decode(&new), Err(HandshakeError::TooNew));

    let long = Hello {
        skipped_len: 0x800,
        ..client_hello()
    }
    .encode(5);
    assert_eq!(Hello::decode(&long), Err(HandshakeError::TooLong));
}

#[test]
fn hello_role_from_sub_version() {
    let role = |sp, sv| {
        Hello {
            sub_protocol: sp,
            sub_version: sv,
            ..client_hello()
        }
        .role()
    };
    assert_eq!(role(SubProtocol::Kdx, 0x10), Some(Role::Client));
    assert_eq!(role(SubProtocol::Kdx, 0x20), Some(Role::Server));
    assert_eq!(role(SubProtocol::Kdx, 0x30), Some(Role::Tracker));
    assert_eq!(role(SubProtocol::Hgc, 0x40), Some(Role::GConsole));
    assert_eq!(role(SubProtocol::Kdx, 0x40), None);
    assert_eq!(role(SubProtocol::Kdx, 0x50), None);
}

#[test]
fn proof_round_trip_and_failure_shape() {
    let table = ChallengeTable::default_table();
    let key = AccountKey::from_password(b"openkdx-lab");
    let n = nonce("d51503a414c2987f0d832176");
    let proof = Proof::compute(&key, &n, &table, 3).unwrap();
    assert_eq!(proof.status, ProofStatus::Ok);
    assert_eq!(&proof.response, &ChallengeTable::DEFAULT[3..23]);
    let bytes = proof.encode();
    assert_eq!(bytes.len(), Proof::LEN);
    assert_eq!(
        &bytes[..4],
        &proof.auth[..4],
        "the key word is sent in the clear"
    );
    assert_ne!(
        &bytes[4..20],
        &proof.auth[4..20],
        "the rest is under the stream"
    );
    assert_eq!(Proof::decode(&bytes).unwrap(), proof);

    assert!(Proof::compute(&key, &n, &table, 12).is_none());

    let failure = Proof::failure(&n, ProofStatus::TooOld);
    let want = u32::from_be_bytes([n[4], n[5], n[6], n[7]]) ^ 0x201f_7790;
    assert_eq!(&failure.auth[..4], &want.to_be_bytes());
    assert_eq!(failure.auth[4..], [0; 16]);
    assert_eq!(failure.response, [0; 20]);
    assert_eq!(Proof::decode(&failure.encode()).unwrap(), failure);

    let mut bad = failure.encode();
    bad[0x28] ^= 0x80;
    assert!(matches!(
        Proof::decode(&bad),
        Err(HandshakeError::InvalidStatus(_))
    ));
}

#[test]
fn challenge_table_bounds() {
    let table = ChallengeTable::default_table();
    assert_eq!(table.len(), 31);
    assert_eq!(table.max_offset(), 11);
    assert!(table.response(11).is_some());
    assert!(table.response(12).is_none());
    let r = table.response(0).unwrap();
    assert!(table.verify(0, &r));
    assert!(!table.verify(1, &r));
}

#[test]
fn challenge_response_matches_capture_when_table_available() {
    // first-connect frames 8 and 9: the client answers the server's challenge
    // 883 and the server answers the client's 1037. The responses are 20 bytes
    // of Haxial's story, so the test holds only their SHA-1 digests.
    let Some(bytes) = lab_table() else {
        eprintln!("skipped: no KDX challenge table (OPENKDX_KDX_TABLE or cache)");
        return;
    };
    use sha1::{Digest, Sha1};
    let table = ChallengeTable::new(bytes);
    assert_eq!(table.len(), 1739);
    assert_eq!(
        hex::encode(Sha1::digest(table.response(883).unwrap())),
        "6214169f17a8b20b8a32e99af8073a8b79cc8460"
    );
    assert_eq!(
        hex::encode(Sha1::digest(table.response(1037).unwrap())),
        "8e3af7c43a50b9b936896d00baaba06046c2a113"
    );
}

fn lab_table() -> Option<Vec<u8>> {
    use openkdx_table::{Cache, Story};
    if let Ok(Some(b)) = openkdx_table::from_env(Story::Kdx) {
        return Some(b);
    }
    Cache::new(Cache::default_dir())
        .get(Story::Kdx)
        .ok()
        .flatten()
}
