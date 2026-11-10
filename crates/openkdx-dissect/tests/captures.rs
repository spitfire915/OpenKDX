//! The seven lab captures, decoded end to end. Lab admin password
//! `openkdx-lab`. Challenge checks run only when a KDX table is available
//! (`OPENKDX_KDX_TABLE` or the openkdx-table cache).

use std::path::{Path, PathBuf};

use openkdx_dissect::audit::Audit;
use openkdx_dissect::connection::{
    Capture, ConnectionKind, Options, Ports, Record, dissect_capture,
};
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
            Record::Message { message: Ok(m), .. } => Some(m.as_ref()),
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
    assert_exact(&cap, "tracker-basic");
    let msgs = messages(&cap, 0);
    let Message::TrackerGroupList(groups) = msgs[1] else {
        panic!("{:?}", msgs[1].kind())
    };
    let names: Vec<&[u8]> = groups.groups.iter().map(|g| g.name.as_slice()).collect();
    assert_eq!(
        names,
        [
            &b"Business"[..],
            b"Chat",
            b"Education",
            b"Games",
            b"General",
            b"Macintosh",
            b"Trackers",
            b"Windows"
        ]
    );
    assert_eq!(groups.groups[4].servers, 1, "General holds the lab server");
    let Message::GetServerList(req) = msgs[2] else {
        panic!("{:?}", msgs[2].kind())
    };
    assert_eq!(req.group, b"General");
    let Message::TrackerServerList(servers) = msgs[3] else {
        panic!("{:?}", msgs[3].kind())
    };
    assert_eq!(servers.servers.len(), 1);
    assert_eq!(servers.servers[0].name, b"New Server");
    assert_eq!(servers.servers[0].address, 0x7f00_0001);
    assert_eq!(servers.servers[0].port, 10700);
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
fn news_basic_is_byte_exact() {
    let cap = load("news-basic");
    assert_eq!(cap.connections.len(), 1);
    assert_clean(&cap, "news-basic");
    assert_exact(&cap, "news-basic");
    assert_eq!(
        kinds(&cap, 0),
        vec![100, 101, 105, 605, 606, 607, 608, 611, 613, 609, 610]
    );
    let msgs = messages(&cap, 0);
    let Message::NewsgroupList(list) = msgs[4] else {
        panic!("{:?}", msgs[4].kind())
    };
    assert_eq!(list.groups.len(), 1);
    assert_eq!(list.groups[0].name, b"General News");
    let Message::NewsMessages(news) = msgs[6] else {
        panic!("{:?}", msgs[6].kind())
    };
    assert_eq!(news.permissions, 7, "admin may post, delete, and read");
    assert_eq!(news.messages.len(), 1);
    assert_eq!(news.messages[0].subject, b"Welcome");
    let Message::PostNewsMessage(post) = msgs[7] else {
        panic!("{:?}", msgs[7].kind())
    };
    assert_eq!(post.subject, b"Lab post");
    assert_eq!(post.body, b"Posted from the lab scenario.");
    let Message::NewsMessagePosted(posted) = msgs[8] else {
        panic!("{:?}", msgs[8].kind())
    };
    assert_eq!(posted.message_count, 2);
    let Message::NewsMessage(reply) = msgs[10] else {
        panic!("{:?}", msgs[10].kind())
    };
    assert_eq!(reply.message.message_id, posted.message_id);
    assert_eq!(reply.message.subject, b"Lab post");
    assert_eq!(reply.message.login, b"admin");
    assert_eq!(reply.message.poster, b"labuser");
}

#[test]
fn files_basic_is_byte_exact() {
    let cap = load("files-basic");
    assert_eq!(cap.connections.len(), 2);
    assert_clean(&cap, "files-basic");
    assert_exact(&cap, "files-basic");
    let main = &cap.connections[0];
    let transfer = &cap.connections[1];
    assert_eq!(main.key.as_ref().unwrap().label, "--password #1");
    assert!(
        transfer
            .key
            .as_ref()
            .unwrap()
            .label
            .starts_with("105 Server Info of"),
        "{}",
        transfer.key.as_ref().unwrap().label
    );
    assert_eq!(kinds(&cap, 0), vec![100, 101, 105, 400, 401, 400, 401]);
    assert_eq!(kinds(&cap, 1), vec![412, 413, 414, 414]);

    let msgs = messages(&cap, 0);
    let Message::FileList(list) = msgs[4] else {
        panic!("{:?}", msgs[4].kind())
    };
    let names: Vec<(&[u8], u64)> = list
        .entries
        .iter()
        .map(|e| (e.name.as_slice(), e.size))
        .collect();
    // readdir order, unsorted; the client sorts for display
    assert_eq!(
        names,
        [
            (&b"tiny.txt"[..], 3),
            (b"Uploads [UL]", 0x1000),
            (b"Admin Dropbox [DB]", 0x1000)
        ]
    );
    assert_eq!(list.entries[1].access_item, 0x2b);
    assert_eq!(list.entries[2].access_item, 0x2c);
    assert_eq!(list.entries[0].kind, b"", "401 carries no type for a .txt");

    let t = messages(&cap, 1);
    let Message::DownloadFile(req) = t[0] else {
        panic!("{:?}", t[0].kind())
    };
    assert_eq!(req.path.to_string(), "tiny.txt");
    let Message::DownloadInfo(info) = t[1] else {
        panic!("{:?}", t[1].kind())
    };
    assert_eq!(info.data_size, 3);
    assert_eq!(info.rsrc_size, 0);
    assert_eq!(info.strings.name, b"tiny.txt");
    assert_eq!(info.strings.second, b"text/plain", "a MIME type for .txt");
    assert_eq!(
        req.rsrc_offset,
        u64::MAX,
        "Client 1.600 asks for no resource fork"
    );
    // the data headers: 3 bytes for the data fork, 0 for the resource fork
    let data: Vec<(u64, bool)> = transfer
        .records
        .iter()
        .filter_map(|r| match r {
            Record::DataHeader { header, .. } => Some((header.len, header.reply)),
            _ => None,
        })
        .collect();
    assert_eq!(data, [(3, true), (0, false)]);
    assert!(
        transfer
            .records
            .iter()
            .any(|r| matches!(r, Record::Data { len: 3, .. }))
    );
}

#[test]
fn all_seven_captures_are_byte_exact() {
    for name in CAPTURES {
        let cap = load(name);
        assert_clean(&cap, name);
        assert_exact(&cap, name);
        for r in &cap.registrations {
            let reg = r
                .registration
                .as_ref()
                .unwrap_or_else(|e| panic!("{name}: registration: {e}"));
            assert_eq!(
                reg.encode(),
                r.datagram,
                "{name}: registration frame {}",
                r.frame
            );
        }
        for c in &cap.connections {
            for r in &c.records {
                if let Record::Message {
                    message,
                    header,
                    frame,
                    ..
                } = r
                {
                    let m = message.as_ref().unwrap();
                    assert!(
                        !matches!(**m, Message::Unknown { .. }),
                        "{name}: frame {frame} {} is not modeled",
                        MessageType(header.kind)
                    );
                }
            }
        }
    }
}

#[test]
fn every_payload_prefix_decodes_without_panic() {
    for name in CAPTURES {
        let cap = load(name);
        for c in &cap.connections {
            for r in &c.records {
                if let Record::Message {
                    header, payload, ..
                } = r
                {
                    for n in 0..payload.len() {
                        let _ = Message::decode(MessageType(header.kind), &payload[..n]);
                    }
                }
            }
        }
        for r in &cap.registrations {
            for n in 0..r.datagram.len() {
                let _ = Registration::decode(&r.datagram[..n]);
            }
        }
    }
}

#[test]
fn export_writes_both_streams() {
    let dir = tempfile::tempdir().unwrap();
    openkdx_dissect::connection::export_streams(
        &capture_path("first-connect"),
        &Ports::default(),
        dir.path(),
    )
    .unwrap();
    let c2s = std::fs::read(dir.path().join("58652.c2s.bin")).unwrap();
    let s2c = std::fs::read(dir.path().join("58652.s2c.bin")).unwrap();
    assert_eq!(c2s.len(), 44 + 44 + 16 + 560);
    assert!(s2c.len() > 36 + 44);
    let (hello, used) = openkdx_proto::handshake::Hello::decode(&c2s).unwrap();
    assert_eq!(used, 44);
    assert_eq!(hello.name, b"admin");
}

#[test]
fn inventory_matches_capture_notes() {
    for name in CAPTURES {
        let cap = load(name);
        let inventory = openkdx_dissect::text::inventory(&cap);
        let notes = std::fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../captures/{name}.md")),
        )
        .unwrap();
        let section = notes
            .split("## Messages\n")
            .nth(1)
            .unwrap_or_else(|| panic!("{name}.md has no '## Messages' section"));
        let section = section.split("\n## ").next().unwrap();
        // the fenced block, after the lead sentence
        let fence = section
            .find("```")
            .unwrap_or_else(|| panic!("{name}.md: no fenced block"));
        assert_eq!(
            section[fence..].trim_end(),
            inventory.trim_end(),
            "{name}.md's Messages section is stale; regenerate it with `openkdx-dissect --inventory`"
        );
    }
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
