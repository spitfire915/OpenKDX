//! The seven lab captures, decoded end to end. Lab admin password
//! `openkdx-lab`. Challenge checks run only when a KDX table is available
//! (`OPENKDX_KDX_TABLE` or the openkdx-table cache).

use std::path::{Path, PathBuf};

use openkdx_dissect::audit::Audit;
use openkdx_dissect::connection::{Capture, ConnectionKind, Options, Record, dissect_capture};
use openkdx_proto::ChallengeTable;
use openkdx_proto::message_type::MessageType;
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

#[test]
fn first_connect_decodes_completely() {
    let cap = load("first-connect");
    assert_eq!(cap.connections.len(), 1);
    let c = &cap.connections[0];
    assert_eq!(c.kind, ConnectionKind::Server);
    assert_eq!(c.key.as_ref().unwrap().label, "--password #1");
    assert_clean(&cap, "first-connect");
    assert_eq!(kinds(&cap, 0), vec![100, 101, 105]);
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
