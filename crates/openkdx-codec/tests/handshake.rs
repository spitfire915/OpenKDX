use bytes::BytesMut;
use openkdx_codec::{HandshakeCodec, HandshakeFrame};
use openkdx_proto::AccountKey;
use openkdx_proto::ChallengeTable;
use openkdx_proto::handshake::{Hello, Proof, SubProtocol};
use tokio_util::codec::{Decoder, Encoder};

fn hello() -> Hello {
    Hello {
        sub_protocol: SubProtocol::Kdx,
        version: 1,
        sub_version: 0x10,
        challenge: 5,
        nonce: [7; 12],
        name: b"admin".to_vec(),
        pad_len: 0,
        skipped_len: 0,
        ignored: [0; 4],
        padding: vec![0; 3],
    }
}

#[test]
fn handshake_codec_waits_for_incomplete_hello_then_proof() {
    let mut codec = HandshakeCodec::new(0xcafe_f00d);
    let mut out = BytesMut::new();
    codec
        .encode(HandshakeFrame::Hello(hello()), &mut out)
        .unwrap();
    assert_eq!(out.len(), 44);
    let proof = Proof::compute(
        &AccountKey::GUEST,
        &[1; 12],
        &ChallengeTable::default_table(),
        3,
    )
    .unwrap();
    codec
        .encode(HandshakeFrame::Proof(proof), &mut out)
        .unwrap();
    assert_eq!(out.len(), 88);

    let mut rx = HandshakeCodec::new(1);
    let mut buf = BytesMut::new();
    buf.extend_from_slice(&out[..20]);
    assert!(rx.decode(&mut buf).unwrap().is_none());
    buf.extend_from_slice(&out[20..44]);
    assert!(matches!(rx.decode(&mut buf).unwrap(), Some(HandshakeFrame::Hello(h)) if h == hello()));
    buf.extend_from_slice(&out[44..60]);
    assert!(rx.decode(&mut buf).unwrap().is_none());
    buf.extend_from_slice(&out[60..]);
    assert!(matches!(rx.decode(&mut buf).unwrap(), Some(HandshakeFrame::Proof(p)) if p == proof));
    assert!(buf.is_empty());
    assert!(rx.is_done());
}
