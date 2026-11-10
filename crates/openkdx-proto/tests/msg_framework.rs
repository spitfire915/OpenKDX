use openkdx_proto::MessageType;
use openkdx_proto::msg::common::{ChatRecord, Color, FilePath, PathName, Time1900, UserRecord};
use openkdx_proto::msg::{Message, Payload, Status, StatusDetail, error_name};
use openkdx_proto::wire::{DecodeError, Reader, Writer};

/// Every type number in the reference spec's messages/README.md.
const ALL_TYPES: &[u32] = &[
    1, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117,
    118, 140, 141, 142, 143, 150, 151, 152, 153, 154, 155, 170, 171, 172, 173, 174, 175, 180, 181,
    182, 183, 184, 185, 186, 187, 188, 189, 190, 191, 200, 201, 202, 203, 204, 205, 206, 207, 208,
    209, 210, 211, 212, 213, 214, 215, 216, 217, 218, 219, 220, 221, 222, 223, 300, 301, 400, 401,
    402, 403, 404, 405, 406, 407, 408, 409, 410, 411, 412, 413, 414, 415, 416, 417, 418, 419, 420,
    421, 422, 423, 450, 451, 452, 453, 502, 503, 504, 505, 506, 507, 508, 509, 510, 511, 512, 513,
    514, 515, 550, 551, 600, 601, 602, 603, 604, 605, 606, 607, 608, 609, 610, 611, 612, 613, 614,
    700, 701, 702, 703, 710, 711, 712, 713, 720, 721, 800, 801, 802, 803, 804, 900, 901, 902, 903,
    904, 905, 1000, 1001, 1002, 1003, 1100, 1101, 1102, 1103, 1104, 1105, 1106, 1107, 4000, 4001,
    4200, 4201,
];

#[test]
fn every_listed_type_has_a_name() {
    let mut names = std::collections::HashSet::new();
    for &t in ALL_TYPES {
        let kind = MessageType(t);
        let name = kind
            .name()
            .unwrap_or_else(|| panic!("type {t} has no name"));
        assert!(kind.is_known());
        assert!(names.insert(name), "duplicate name {name:?} for type {t}");
    }
    assert_eq!(MessageType::SET_IDENTITY, MessageType(100));
    assert_eq!(MessageType::DOWNLOAD_FILE_DATA, MessageType(414));
    assert_eq!(MessageType(100).name(), Some("Set Identity"));
    assert_eq!(MessageType(414).name(), Some("Download File (D)"));
    assert_eq!(MessageType(118).name(), Some("(unnamed; license report)"));
    assert_eq!(MessageType(9999).name(), None);
    assert!(!MessageType(9999).is_known());
    assert_eq!(MessageType(100).to_string(), "100 Set Identity");
    assert_eq!(MessageType(9999).to_string(), "9999 (unknown)");
}

#[test]
fn status_round_trip_and_error_name() {
    let tracker_form = [0x00, 0x01, 0x00, 0x14, 0x00, 0x00, 0x02, 0xc6];
    let s = Status::decode(&tracker_form).unwrap();
    assert_eq!(s.code, 0x10014);
    assert_eq!(s.request_type_or_value, 710);
    assert_eq!(s.detail, None);
    assert_eq!(s.to_bytes(), tracker_form);
    assert_eq!(
        error_name(0x10014),
        Some("Access Denied (a missing privilege)")
    );
    assert_eq!(
        error_name(0x40071),
        Some("The network protocol is unknown.")
    );
    assert_eq!(error_name(0x20065), Some("memory errors"));
    assert_eq!(error_name(0x640005), None);

    let server_form = [
        0x00, 0x01, 0x00, 0x0a, 0x00, 0x00, 0x01, 0x90, 0x00, 0x00, 0x00, 0x46, 0x00, 0x00, 0x00,
        0x98, 0, 0, 0, 7, 0, 0, 0, 0,
    ];
    let s = Status::decode(&server_form).unwrap();
    assert_eq!(
        s.detail,
        Some(StatusDetail {
            module: 0x46,
            line: 0x98,
            extra: 7,
            ignored: 0
        })
    );
    assert_eq!(s.to_bytes(), server_form);

    assert!(matches!(
        Status::decode(&server_form[..20]),
        Err(DecodeError::Truncated { .. })
    ));
    assert_eq!(Status::KIND, MessageType::STATUS);
}

#[test]
fn message_decode_dispatches_status_and_unknown() {
    let m = Message::decode(MessageType(1), &[0, 1, 0, 0x14, 0, 0, 2, 0xc6]).unwrap();
    assert!(matches!(m, Message::Status(_)));
    assert_eq!(m.kind(), MessageType(1));
    assert_eq!(m.encode(), vec![0, 1, 0, 0x14, 0, 0, 2, 0xc6]);

    let bytes = vec![1, 2, 3];
    let m = Message::decode(MessageType(9999), &bytes).unwrap();
    assert!(
        matches!(&m, Message::Unknown { kind, bytes: b } if *kind == MessageType(9999) && *b == bytes)
    );
    assert_eq!(m.encode(), bytes);
}

#[test]
fn file_path_root_is_empty_and_delimited_root_is_two_zero_bytes() {
    let root = FilePath::decode(&[]).unwrap();
    assert!(root.names.is_empty());
    assert_eq!(root.to_bytes(), Vec::<u8>::new());

    let mut r = Reader::new(&[0, 0, 0xaa]);
    let d = FilePath::decode_delimited(&mut r).unwrap();
    assert!(d.names.is_empty());
    assert_eq!(r.remaining(), 1);
    let mut w = Writer::new();
    d.encode_delimited(&mut w);
    assert_eq!(w.into_inner(), vec![0, 0]);

    // a count of 0 as a whole payload is the malformed `400:0000`
    assert!(FilePath::decode(&[0, 0]).is_err());
}

#[test]
fn file_path_example_from_spec() {
    let mut bytes = vec![0x00, 0x01, 0x00, 0x00, 0x0c];
    bytes.extend_from_slice(b"Uploads [UL]");
    let p = FilePath::decode(&bytes).unwrap();
    assert_eq!(
        p.names,
        vec![PathName {
            ignored: 0,
            name: b"Uploads [UL]".to_vec()
        }]
    );
    assert_eq!(p.to_bytes(), bytes);
    assert_eq!(p.to_string(), "Uploads [UL]");
    let two = FilePath {
        names: vec![
            PathName {
                ignored: 0,
                name: b"a".to_vec(),
            },
            PathName {
                ignored: 0,
                name: b"b".to_vec(),
            },
        ],
    };
    assert_eq!(two.to_string(), "a/b");
}

#[test]
fn time_1900_converts_to_unix() {
    assert_eq!(Time1900(2_208_988_800_000).to_unix_ms(), 0);
    assert_eq!(Time1900(0).to_unix_ms(), -2_208_988_800_000);
}

fn sample_user() -> UserRecord {
    UserRecord {
        user_id: 1,
        status: 0,
        ignored_05: [0; 5],
        client_port: 10600,
        text_color: Color { r: 0, g: 0, b: 0 },
        background_color: Color {
            r: 0xff,
            g: 0xcc,
            b: 0,
        },
        account_color: Color {
            r: 0xff,
            g: 0,
            b: 0,
        },
        flags: 0,
        ignored_1b: 0,
        icon: vec![0x11; 0x20b],
        extra: vec![],
        nick: b"labuser".to_vec(),
        info: vec![],
        padding: vec![],
    }
}

#[test]
fn user_record_without_padding_is_558_bytes_and_with_it_560() {
    let user = sample_user();
    let mut w = Writer::new();
    user.encode(&mut w);
    let bytes = w.into_inner();
    assert_eq!(bytes.len(), 0x1c + 0x20b + 7);
    assert_eq!(bytes.len(), 558); // 100 Set Identity in first-connect
    let mut r = Reader::new(&bytes);
    let back = UserRecord::decode(&mut r).unwrap();
    r.finish().unwrap();
    assert_eq!(back, user);

    let padded = UserRecord {
        padding: vec![0, 0],
        ..sample_user()
    };
    let mut w = Writer::new();
    padded.encode(&mut w);
    let bytes = w.into_inner();
    assert_eq!(bytes.len(), 560); // 101 User List in first-connect
    let mut r = Reader::new(&bytes);
    assert_eq!(UserRecord::decode(&mut r).unwrap(), padded);
    r.finish().unwrap();
}

#[test]
fn user_record_decode_in_a_list_keeps_padding_between_records() {
    let a = UserRecord {
        padding: vec![0, 0],
        ..sample_user()
    };
    let b = UserRecord {
        user_id: 2,
        nick: b"guestuser2".to_vec(),
        padding: vec![0, 0, 0],
        ..sample_user()
    };
    let mut w = Writer::new();
    a.encode(&mut w);
    b.encode(&mut w);
    let bytes = w.into_inner();
    let mut r = Reader::new(&bytes);
    assert_eq!(UserRecord::decode(&mut r).unwrap(), a);
    assert_eq!(UserRecord::decode(&mut r).unwrap(), b);
    r.finish().unwrap();
}

#[test]
fn chat_record_round_trip() {
    let chat = ChatRecord {
        chat_id: 1,
        key: 0xdead_beef,
        ignored_08: [0; 12],
        join_classes: 0xffff_ffff,
        send_classes: 0xffff_ffff,
        topic_classes: 0x8000_0000,
        ignored_20: [0; 8],
        flags: 0,
        ignored_2e: [0; 2],
        name: b"Public Chat".to_vec(),
        description: b"For all users".to_vec(),
        irc_server: b"irc.example:6667".to_vec(),
        irc_channel: b"#kdx".to_vec(),
    };
    let mut w = Writer::new();
    chat.encode(&mut w);
    let bytes = w.into_inner();
    assert_eq!(bytes.len(), 0x30 + 11 + 13 + 16 + 4);
    let mut r = Reader::new(&bytes);
    assert_eq!(ChatRecord::decode(&mut r).unwrap(), chat);
    r.finish().unwrap();
    assert!(!chat.is_private() && !chat.is_voice());
}
