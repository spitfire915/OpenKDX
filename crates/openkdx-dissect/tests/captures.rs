//! The seven lab captures, decoded end to end. Lab admin password
//! `openkdx-lab`. Challenge checks run only when a KDX table is available
//! (`OPENKDX_KDX_TABLE` or the openkdx-table cache).

use std::path::{Path, PathBuf};

use openkdx_dissect::audit::Audit;
use openkdx_dissect::connection::{Capture, ConnectionKind, Options, Record, dissect_capture};
use openkdx_proto::ChallengeTable;
use openkdx_proto::message_type::MessageType;
use openkdx_proto::msg::Message;
use openkdx_proto::registration::Registration;

const CAPTURES: [&str; 7] = [
    "first-connect",
    "guest-login",
    "chat-basic",
    "pm-basic",
    "news-basic",
    "files-basic",
    "tracker-basic",
];

fn capture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../captures")
        .join(format!("{name}.pcapng"))
}

fn table() -> Option<ChallengeTable> {
    use openkdx_table::{Cache, Story};
    let bytes = match openkdx_table::from_env(Story::Kdx) {
        Ok(Some(b)) => Some(b),
        _ => Cache::new(Cache::default_dir())
            .get(Story::Kdx)
            .ok()
            .flatten(),
    };
    bytes.map(ChallengeTable::new)
}

fn opts() -> Options {
    Options {
        passwords: vec![b"openkdx-lab".to_vec()],
        table: table(),
        ..Options::default()
    }
}

fn load(name: &str) -> Capture {
    dissect_capture(&capture_path(name), &opts()).unwrap()
}

fn kinds(cap: &Capture, conn: usize) -> Vec<u32> {
    cap.connections[conn]
        .records
        .iter()
        .filter_map(|r| match r {
            Record::Message { header, .. } | Record::DataHeader { header, .. } => Some(header.kind),
            _ => None,
        })
        .collect()
}

fn assert_clean(cap: &Capture, name: &str) {
    for c in &cap.connections {
        assert!(
            c.key.is_some(),
            "{name}: connection {} has no key",
            c.client
        );
        for r in &c.records {
            match r {
                Record::Stopped { reason, frame, .. } => {
                    panic!(
                        "{name}: connection {} stopped at frame {frame}: {reason}",
                        c.client
                    )
                }
                Record::Proof {
                    auth_ok,
                    challenge_ok,
                    frame,
                    ..
                } => {
                    assert_eq!(*auth_ok, Some(true), "{name}: proof auth at frame {frame}");
                    if cap.table_present {
                        assert_eq!(
                            *challenge_ok,
                            Some(true),
                            "{name}: challenge at frame {frame}"
                        );
                    }
                }
                _ => {}
            }
        }
    }
}

/// Every message in `cap` decodes and re-encodes byte for byte.
fn assert_exact(cap: &Capture, name: &str) {
    for c in &cap.connections {
        for r in &c.records {
            if let Record::Message {
                message,
                payload,
                frame,
                header,
                ..
            } = r
            {
                let a = openkdx_dissect::audit::audit(message, payload);
                assert!(
                    a.is_exact(),
                    "{name}: frame {frame} {}: {a}",
                    MessageType(header.kind)
                );
            }
        }
    }
}

/// The decoded messages of connection `conn`, in order.
fn messages(cap: &Capture, conn: usize) -> Vec<&Message> {
    cap.connections[conn]
        .records
        .iter()
        .filter_map(|r| match r {
            Record::Message { message: Ok(m), .. } => Some(m),
            _ => None,
        })
        .collect()
}

#[test]
fn first_connect_decodes_completely() {
    let cap = load("first-connect");
    assert_eq!(cap.connections.len(), 1);
    let c = &cap.connections[0];
    assert_eq!(c.kind, ConnectionKind::Server);
    assert_eq!(c.key.as_ref().unwrap().label, "--password #1");
    assert_clean(&cap, "first-connect");
    assert_eq!(kinds(&cap, 0), vec![100, 101, 105]);
    assert_exact(&cap, "first-connect");
    let msgs = messages(&cap, 0);
    let Message::SetIdentity(set) = msgs[0] else {
        panic!("first message is {:?}", msgs[0].kind());
    };
    assert_eq!(set.user.user_id, 0);
    assert_eq!(set.user.nick, b"labuser");
    assert_eq!(set.user.client_port, 10600);
    assert_eq!(
        (
            set.user.background_color.r,
            set.user.background_color.g,
            set.user.background_color.b
        ),
        (0xff, 0xcc, 0)
    );
    assert!(set.user.padding.is_empty());
    let Message::UserList(list) = msgs[1] else {
        panic!("second message is {:?}", msgs[1].kind());
    };
    assert_eq!(list.users.len(), 1);
    assert_eq!(list.users[0].user_id, 1);
    assert_eq!(list.users[0].padding, vec![0, 0]);
    assert_eq!(
        (
            list.users[0].account_color.r,
            list.users[0].account_color.g,
            list.users[0].account_color.b
        ),
        (0xff, 0, 0),
        "admin's class colour"
    );
    let Message::ServerInfo(info) = msgs[2] else {
        panic!("third message is {:?}", msgs[2].kind());
    };
    assert_eq!(info.name, b"New Server");
    assert_eq!(info.version, 0x0654);
    assert_eq!(info.user_id, 1);
    assert_eq!(info.can_modify_settings, 1);
    assert_eq!(
        &info.session_key[..16],
        &c.key.as_ref().unwrap().key.as_bytes()[..16]
    );
    let hellos: Vec<_> = c
        .records
        .iter()
        .filter_map(|r| match r {
            Record::Hello { hello, frame, .. } => {
                Some((*frame, hello.challenge, hello.name.clone()))
            }
            _ => None,
        })
        .collect();
    assert_eq!(hellos, vec![(4, 1037, b"admin".to_vec()), (6, 883, vec![])]);
}

#[test]
fn guest_login_uses_the_guest_key() {
    let cap = load("guest-login");
    assert_eq!(cap.connections.len(), 1);
    assert_eq!(
        cap.connections[0].key.as_ref().unwrap().label,
        "empty password"
    );
    assert_clean(&cap, "guest-login");
    assert_eq!(kinds(&cap, 0), vec![100, 101, 105]);
    assert_exact(&cap, "guest-login");
    let Message::ServerInfo(info) = messages(&cap, 0)[2] else {
        panic!("no server info");
    };
    assert_eq!(info.can_modify_settings, 0);
    // The session key's first half is the account key's; the second half is
    // per session (so is the admin's in first-connect).
    assert_eq!(
        &info.session_key[..16],
        &openkdx_proto::AccountKey::GUEST.as_bytes()[..16]
    );
    assert_ne!(
        &info.session_key[16..],
        &openkdx_proto::AccountKey::GUEST.as_bytes()[16..]
    );
}

#[test]
fn tracker_basic_registration_is_byte_exact() {
    let cap = load("tracker-basic");
    assert_eq!(cap.registrations.len(), 1);
    let reg = &cap.registrations[0];
    assert_eq!(reg.frame, 1);
    assert_eq!(reg.datagram.len(), 84);
    let decoded = reg.registration.as_ref().unwrap();
    assert_eq!(decoded.name, b"New Server");
    assert_eq!(decoded.server_port, 10700);
    assert_eq!(decoded.encode(), reg.datagram);
    assert_eq!(Registration::decode(&reg.datagram).unwrap(), *decoded);

    assert_eq!(cap.connections.len(), 1);
    let c = &cap.connections[0];
    assert_eq!(c.kind, ConnectionKind::Tracker);
    assert_eq!(c.key.as_ref().unwrap().label, "empty password");
    assert_clean(&cap, "tracker-basic");
    assert_eq!(kinds(&cap, 0), vec![700, 701, 702, 703]);
}

#[test]
fn chat_basic_is_byte_exact() {
    let cap = load("chat-basic");
    assert_eq!(cap.connections.len(), 1);
    assert_clean(&cap, "chat-basic");
    assert_exact(&cap, "chat-basic");
    assert_eq!(
        kinds(&cap, 0),
        vec![
            100, 101, 105, 214, 215, 202, 203, 207, 208, 207, 208, 209, 210
        ]
    );
    let msgs = messages(&cap, 0);
    let Message::ChatList(list) = msgs[4] else {
        panic!("{:?}", msgs[4].kind())
    };
    let names: Vec<&[u8]> = list.chats.iter().map(|c| c.name.as_slice()).collect();
    // creation order; the client sorts them for display
    assert_eq!(names, [&b"Public Chat"[..], b"Voice Chat", b"Admin Lounge"]);
    let Message::ChatWindowInfo(info) = msgs[6] else {
        panic!("{:?}", msgs[6].kind())
    };
    assert_eq!(info.name, b"Public Chat");
    assert_eq!(info.members, vec![1]);
    let texts: Vec<(&[u8], bool)> = msgs
        .iter()
        .filter_map(|m| match m {
            Message::ReceiveChatText(t) => Some((t.text.as_slice(), t.is_action())),
            _ => None,
        })
        .collect();
    // the client strips `/me ` and sets the action flag instead
    assert_eq!(
        texts,
        [(&b"hello from the lab"[..], false), (b"waves", true)]
    );
    let Message::ChatTopicChanged(topic) = msgs[12] else {
        panic!("{:?}", msgs[12].kind())
    };
    assert_eq!(topic.topic, b"lab topic");
    assert_eq!(topic.nick, b"labuser");
}

#[test]
fn pm_basic_is_byte_exact() {
    let cap = load("pm-basic");
    assert_eq!(cap.connections.len(), 2);
    assert_clean(&cap, "pm-basic");
    assert_exact(&cap, "pm-basic");
    // flows are ordered by first frame: the Linux guest connected first
    assert_eq!(
        cap.connections[0].key.as_ref().unwrap().label,
        "empty password"
    );
    assert_eq!(
        cap.connections[1].key.as_ref().unwrap().label,
        "--password #1"
    );
    assert_eq!(kinds(&cap, 0), vec![100, 101, 105, 103, 301, 300]);
    assert_eq!(kinds(&cap, 1), vec![100, 101, 105, 300, 301]);
    let admin = messages(&cap, 1);
    let Message::UserList(list) = admin[1] else {
        panic!("{:?}", admin[1].kind())
    };
    assert_eq!(list.users.len(), 2);
    let Message::SendPrivateMessage(pm) = admin[3] else {
        panic!("{:?}", admin[3].kind())
    };
    assert_eq!(pm.text, b"hi from admin");
    let Message::IncomingPrivateMessage(pm) = admin[4] else {
        panic!("{:?}", admin[4].kind())
    };
    assert_eq!(pm.text, b"hello back");
}

#[test]
fn every_capture_has_no_stopped_connection() {
    for name in CAPTURES {
        let cap = load(name);
        assert!(!cap.connections.is_empty(), "{name}: no connections");
        assert_clean(&cap, name);
    }
}

#[test]
fn unknown_payloads_audit_as_unknown_not_mismatch() {
    // Until a message is modeled, its audit says so instead of failing.
    let cap = load("first-connect");
    for r in &cap.connections[0].records {
        if let Record::Message {
            message, payload, ..
        } = r
        {
            let a = openkdx_dissect::audit::audit(message, payload);
            assert!(
                matches!(a, Audit::Exact | Audit::Unknown),
                "{}: {a:?}",
                MessageType(r.kind().unwrap())
            );
        }
    }
}
