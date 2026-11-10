//! The session codec against the captured streams of `captures/`.

use std::path::{Path, PathBuf};

use bytes::{Bytes, BytesMut};
use openkdx_codec::{CodecError, Event, Item, SessionCodec};
use openkdx_dissect::capture::{Flow, parse_packets, read_capture, tcp_flows};
use openkdx_dissect::connection::{Options, Record, dissect_capture};
use openkdx_proto::AccountKey;
use openkdx_proto::handshake::{Hello, Proof};
use openkdx_proto::session::{CLIENT_RECV_CAP, Header, SERVER_RECV_CAP, SessionCipher};
use tokio_util::codec::{Decoder, Encoder};

fn capture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../captures")
        .join(format!("{name}.pcapng"))
}

fn flows(name: &str) -> Vec<Flow> {
    let frames = read_capture(&capture_path(name)).unwrap();
    let packets = parse_packets(&frames);
    let mut flows = tcp_flows(&packets, |p| p == 10700 || p == 10800);
    flows.sort_by_key(|f| f.first_frame);
    flows
}

/// The session bytes of one direction: everything after the hello and proof.
fn session_bytes(stream: &[u8]) -> &[u8] {
    let (_, used) = Hello::decode(stream).unwrap();
    &stream[used + Proof::LEN..]
}

/// Decodes `data` through `codec`, fed in chunks of `chunk` bytes.
fn decode_all(codec: &mut SessionCodec, data: &[u8], chunk: usize) -> Vec<Event> {
    let mut buf = BytesMut::new();
    let mut events = Vec::new();
    for piece in data.chunks(chunk.max(1)) {
        buf.extend_from_slice(piece);
        while let Some(ev) = codec.decode(&mut buf).unwrap() {
            events.push(ev);
        }
    }
    assert!(codec.decode_eof(&mut buf).unwrap().is_none());
    assert!(buf.is_empty(), "{} bytes left over", buf.len());
    events
}

fn message_events(events: &[Event]) -> Vec<(Header, Bytes)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Message { header, payload } => Some((*header, payload.clone())),
            _ => None,
        })
        .collect()
}

#[test]
fn session_codec_decodes_first_connect_streams() {
    let flow = &flows("first-connect")[0];
    let key = AccountKey::from_password(b"openkdx-lab");
    let expected = dissect_capture(
        &capture_path("first-connect"),
        &Options {
            passwords: vec![b"openkdx-lab".to_vec()],
            ..Options::default()
        },
    )
    .unwrap();
    let records: Vec<(Header, Vec<u8>)> = expected.connections[0]
        .records
        .iter()
        .filter_map(|r| match r {
            Record::Message {
                header, payload, ..
            } => Some((*header, payload.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(records.len(), 3);

    for chunk in [1usize, 7, 1 << 20] {
        let mut c2s = SessionCodec::new(&key, SERVER_RECV_CAP);
        let mut s2c = SessionCodec::new(&key, CLIENT_RECV_CAP);
        let mut got = message_events(&decode_all(&mut c2s, session_bytes(&flow.c2s.data), chunk));
        got.extend(message_events(&decode_all(
            &mut s2c,
            session_bytes(&flow.s2c.data),
            chunk,
        )));
        let got: Vec<(Header, Vec<u8>)> = got.into_iter().map(|(h, p)| (h, p.to_vec())).collect();
        assert_eq!(got, records, "chunk size {chunk}");
    }
}

#[test]
fn session_codec_encodes_back_byte_exact() {
    let flow = &flows("first-connect")[0];
    let key = AccountKey::from_password(b"openkdx-lab");
    for (data, cap) in [
        (session_bytes(&flow.c2s.data), SERVER_RECV_CAP),
        (session_bytes(&flow.s2c.data), CLIENT_RECV_CAP),
    ] {
        let mut codec = SessionCodec::new(&key, cap);
        let events = decode_all(&mut codec, data, 4096);
        let mut out = BytesMut::new();
        for (header, payload) in message_events(&events) {
            codec
                .encode(Item::Message { header, payload }, &mut out)
                .unwrap();
        }
        assert_eq!(&out[..], data);
    }
}

#[test]
fn transfer_data_mode_round_trip() {
    let all = flows("files-basic");
    // The transfer connection: its key is the parent's 105 session key.
    let parent = &all[0];
    let transfer = &all[1];
    let key = AccountKey::from_password(b"openkdx-lab");
    let cipher = SessionCipher::new(&key);
    // find the 105 in the parent's server stream
    let s2c = session_bytes(&parent.s2c.data);
    let mut off = 0;
    let mut session_key = None;
    while off + 16 <= s2c.len() {
        let block: &[u8; 16] = s2c[off..off + 16].try_into().unwrap();
        let header = cipher.open_header(block, CLIENT_RECV_CAP).unwrap();
        off += 16;
        let payload = cipher
            .open_payload(&header, &s2c[off..off + header.padded_len()])
            .unwrap();
        off += header.padded_len();
        if header.kind == 105 {
            session_key = Some(AccountKey(payload[..32].try_into().unwrap()));
        }
    }
    let session_key = session_key.expect("105 Server Info");

    let mut codec = SessionCodec::new(&session_key, CLIENT_RECV_CAP);
    let data = session_bytes(&transfer.s2c.data);
    let events = decode_all(&mut codec, data, 5);
    let shape: Vec<String> = events
        .iter()
        .map(|e| match e {
            Event::Message { header, .. } => format!("msg {}", header.kind),
            Event::DataStart(h) => format!("start {} {}", h.kind, h.len),
            Event::Data(b) => format!("data {}", b.len()),
            Event::DataEnd => "end".to_string(),
        })
        .collect();
    assert_eq!(
        shape,
        [
            "msg 413",
            "start 414 3",
            "data 3",
            "end",
            "start 414 0",
            "end"
        ]
    );
    let Event::Data(bytes) = &events[2] else {
        panic!()
    };
    assert_eq!(&bytes[..], b"abc");

    // encode the same sequence and compare with the captured ciphertext
    let mut out = BytesMut::new();
    let mut headers = events.iter().filter_map(|e| match e {
        Event::Message { header, payload } => Some(Item::Message {
            header: *header,
            payload: payload.clone(),
        }),
        Event::DataStart(h) => Some(Item::DataHeader(*h)),
        _ => None,
    });
    codec.encode(headers.next().unwrap(), &mut out).unwrap(); // 413
    codec.encode(headers.next().unwrap(), &mut out).unwrap(); // 414 header, 3 bytes
    codec
        .encode(Item::Data(Bytes::from_static(b"ab")), &mut out)
        .unwrap();
    codec
        .encode(Item::Data(Bytes::from_static(b"c")), &mut out)
        .unwrap();
    codec.encode(headers.next().unwrap(), &mut out).unwrap(); // 414 header, 0 bytes
    // Everything is byte-exact except the padding of the data block: Server
    // 1.620 fills it with a value that is not the txid LCG (the reference spec
    // says so too, and gives no formula), so that one block is compared
    // decrypted. Layout: 413 (16 + 96), 414 header (16), data block (16), 414 header (16).
    let data_block = 16 + 96 + 16;
    assert_eq!(&out[..data_block], &data[..data_block]);
    assert_eq!(&out[data_block + 16..], &data[data_block + 16..]);
    let cipher = SessionCipher::new(&session_key);
    let mut ours = out[data_block..data_block + 16].to_vec();
    let mut theirs = data[data_block..data_block + 16].to_vec();
    cipher.decrypt_blocks(&mut ours);
    cipher.decrypt_blocks(&mut theirs);
    assert_eq!(&ours[..3], b"abc");
    assert_eq!(&theirs[..3], b"abc");
}

#[test]
fn rejects_len_over_cap_before_buffering() {
    let key = AccountKey::GUEST;
    let cipher = SessionCipher::new(&key);
    let block = cipher.seal_header(Header::request(1, 100, 0x20001));
    let mut codec = SessionCodec::new(&key, 0x20000);
    let mut buf = BytesMut::from(&block[..]);
    let err = codec.decode(&mut buf).unwrap_err();
    assert!(matches!(err, CodecError::Session(_)), "{err}");
}

#[test]
fn data_overrun_is_an_error() {
    let key = AccountKey::GUEST;
    let mut codec = SessionCodec::new(&key, 0x20000);
    let mut out = BytesMut::new();
    codec
        .encode(Item::DataHeader(Header::request(1, 414, 2)), &mut out)
        .unwrap();
    let err = codec
        .encode(Item::Data(Bytes::from_static(b"abc")), &mut out)
        .unwrap_err();
    assert!(
        matches!(
            err,
            CodecError::DataOverrun {
                sent: 3,
                declared: 2
            }
        ),
        "{err}"
    );
}
