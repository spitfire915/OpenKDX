//! The seven lab captures, decoded end to end. Lab admin password
//! `openkdx-lab`. Challenge checks run only when a KDX table is available
//! (`OPENKDX_KDX_TABLE` or the openkdx-table cache).

use std::path::{Path, PathBuf};

use openkdx_dissect::audit::Audit;
use openkdx_dissect::connection::{
    Capture, ConnectionKind, Dir, Options, Ports, Record, dissect_capture,
};
use openkdx_proto::ChallengeTable;
use openkdx_proto::message_type::MessageType;
use openkdx_proto::msg::Message;
use openkdx_proto::msg::admin::HistoryKind;
use openkdx_proto::registration::Registration;

const CAPTURES: [&str; 15] = [
    "upload-resume",
    "accounts-classes",
    "folder-access",
    "server-settings",
    "broadcast",
    "disconnect-ban",
    "server-history",
    "connection-monitor",
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
        // The lab password, and the wrong one `disconnect-ban` tries.
        passwords: vec![b"openkdx-lab".to_vec(), b"wrong-password".to_vec()],
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
                    proof,
                    auth_ok,
                    challenge_ok,
                    frame,
                    ..
                } => {
                    if proof.status != openkdx_proto::handshake::ProofStatus::Ok {
                        // A failure proof: a zeroed auth and response.
                        assert_eq!(*auth_ok, None, "{name}: failure proof at frame {frame}");
                        continue;
                    }
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
        .unwrap()
        .replace("\r\n", "\n"); // a CRLF checkout on Windows
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

#[test]
fn upload_resume_is_byte_exact() {
    let cap = load("upload-resume");
    assert_clean(&cap, "upload-resume");
    assert_exact(&cap, "upload-resume");
    // main, cut transfer, main (ls), resumed transfer's main..., etc.
    assert!(cap.connections.len() >= 4, "{}", cap.connections.len());
    // The first transfer: 415, 416 (offset 0), a 417 header for the whole
    // file, 150000 bytes, then the client closes.
    let cut = &cap.connections[1];
    assert_eq!(kinds(&cap, 1), vec![415, 416, 417]);
    let cut_record = cut
        .records
        .iter()
        .find_map(|r| match r {
            Record::DataCut {
                captured, declared, ..
            } => Some((*captured, *declared)),
            _ => None,
        })
        .expect("the cut transfer is recorded as DataCut");
    assert_eq!(cut_record, (150_000, 262_144));
    let t = messages(&cap, 1);
    let Message::UploadFile(up) = t[0] else {
        panic!("{:?}", t[0].kind())
    };
    assert_eq!(up.data_size, 262_144);
    assert_eq!(up.strings.name, b"big.bin");
    assert_eq!(up.strings.second, b"exttype/bin");
    let Message::UploadReply(r) = t[1] else {
        panic!("{:?}", t[1].kind())
    };
    assert_eq!(r.data_offset, 0);

    // The resumed transfer: 416 offers a multiple of 0x1000, the client
    // sends the rest under one 417 header (reply to 415's txid), an empty
    // 417 (fresh txid), and gets 418.
    let resumed = cap
        .connections
        .iter()
        .enumerate()
        .skip(2)
        .find(|(i, _)| kinds(&cap, *i).starts_with(&[415, 416]))
        .map(|(i, _)| i)
        .expect("a resumed upload");
    assert_eq!(kinds(&cap, resumed), vec![415, 416, 417, 417, 418]);
    let t = messages(&cap, resumed);
    let Message::UploadReply(r) = t[1] else {
        panic!("{:?}", t[1].kind())
    };
    assert_eq!(r.data_offset, 131_072);
    let headers: Vec<(u64, bool)> = cap.connections[resumed]
        .records
        .iter()
        .filter_map(|r| match r {
            Record::DataHeader { header, .. } => Some((header.len, header.reply)),
            _ => None,
        })
        .collect();
    assert_eq!(headers, vec![(262_144 - 131_072, true), (0, false)]);
    let (txid_415, txid_417) = {
        let mut it = cap.connections[resumed]
            .records
            .iter()
            .filter_map(|r| match r {
                Record::Message { header, .. } | Record::DataHeader { header, .. } => Some(header),
                _ => None,
            });
        let a = it.next().unwrap().txid;
        let c = it.nth(1).unwrap().txid;
        (a, c)
    };
    assert_eq!(txid_415, txid_417, "the data-fork 417 answers 415");

    // A download of the whole file at the end, on its own transfer connection.
    let down = cap
        .connections
        .iter()
        .enumerate()
        .find(|(i, _)| kinds(&cap, *i).starts_with(&[412]))
        .map(|(i, _)| i)
        .expect("a download");
    assert_eq!(kinds(&cap, down), vec![412, 413, 414, 414]);
}

#[test]
fn accounts_classes_is_byte_exact() {
    let cap = load("accounts-classes");
    assert_clean(&cap, "accounts-classes");
    assert_exact(&cap, "accounts-classes");
    assert_eq!(cap.connections.len(), 1);
    assert_eq!(
        kinds(&cap, 0),
        vec![
            100, 101, 105, 118, 505, 506, 502, 503, 512, 513, 507, 1, 509, 510, 509, 510, 511, 1,
            509, 510, 508, 515, 512, 513
        ]
    );
    let msgs = messages(&cap, 0);
    let Message::Status(s) = msgs[11] else {
        panic!("{:?}", msgs[11].kind())
    };
    assert_eq!(
        (s.code, s.request_type_or_value),
        (0, 507),
        "507 → success Status 0x1fb"
    );
    let Message::AccountInfo(info) = msgs[13] else {
        panic!("{:?}", msgs[13].kind())
    };
    assert_eq!(info.account.login, b"tester");
    assert!(info.account.has_password());
    assert_eq!(info.account.editor, b"admin", "510 carries the last editor");
    let Message::SetAccount(set) = msgs[16] else {
        panic!("{:?}", msgs[16].kind())
    };
    assert_eq!(set.account.name, b"Tester 2");
    assert_eq!(
        set.account.editor, b"tester",
        "511 names the account in editor"
    );
    let Message::AccountDeleted(d) = msgs[21] else {
        panic!("{:?}", msgs[21].kind())
    };
    assert_eq!(d.login, b"tester");
}

#[test]
fn folder_access_is_byte_exact() {
    let cap = load("folder-access");
    assert_clean(&cap, "folder-access");
    assert_exact(&cap, "folder-access");
    assert_eq!(
        kinds(&cap, 0),
        vec![
            100, 101, 105, 118, 1106, 1107, 1103, 1104, 1100, 1, 1103, 1104, 402, 1, 400, 401,
            1101, 1102, 1106, 1107
        ]
    );
    let msgs = messages(&cap, 0);
    let Message::FolderAccessInfo(ul) = msgs[7] else {
        panic!("{:?}", msgs[7].kind())
    };
    assert_eq!(ul.item.name, b"UL");
    assert_eq!(ul.item.icon, 0x2b);
    let Message::Status(s) = msgs[9] else {
        panic!("{:?}", msgs[9].kind())
    };
    assert_eq!((s.code, s.request_type_or_value), (0, 1100));
    let Message::FileList(list) = msgs[15] else {
        panic!("{:?}", msgs[15].kind())
    };
    assert!(
        list.entries
            .iter()
            .any(|e| e.name == b"Pictures [Pictures]" && e.is_folder()),
        "{:?}",
        list.entries
            .iter()
            .map(|e| String::from_utf8_lossy(&e.name).into_owned())
            .collect::<Vec<_>>()
    );
}

#[test]
fn server_settings_is_byte_exact() {
    let cap = load("server-settings");
    assert_clean(&cap, "server-settings");
    assert_exact(&cap, "server-settings");
    assert_eq!(cap.connections.len(), 3);
    assert_eq!(
        kinds(&cap, 0),
        vec![100, 101, 105, 118, 170, 171, 170, 171, 172, 173, 170, 171]
    );
    let msgs = messages(&cap, 0);
    let Message::ServerSettingsInfo(before) = msgs[5] else {
        panic!("{:?}", msgs[5].kind())
    };
    assert_eq!(before.settings.main.name, b"New Server");
    let Message::SetServerSettings(set) = msgs[8] else {
        panic!("{:?}", msgs[8].kind())
    };
    assert_eq!(set.settings.main.name, b"OpenKDX Lab");
    assert_eq!(
        set.settings.to_bytes()[0x70 + 0x52],
        11,
        "the length byte at 0x52"
    );
    assert_eq!(
        set.settings.table()[7..],
        before.settings.table()[7..],
        "sections untouched"
    );
    let Message::ServerSettingsInfo(after) = msgs[11] else {
        panic!("{:?}", msgs[11].kind())
    };
    assert_eq!(after.settings.main.name, b"OpenKDX Lab");
    // The second login sees the new name in 105.
    let m2 = messages(&cap, 1);
    let Message::ServerInfo(info) = m2[2] else {
        panic!("{:?}", m2[2].kind())
    };
    assert_eq!(info.name, b"OpenKDX Lab");
    assert_eq!(kinds(&cap, 2), vec![100, 101, 105, 118, 170, 171, 172, 173]);
}

#[test]
fn broadcast_is_byte_exact() {
    let cap = load("broadcast");
    assert_clean(&cap, "broadcast");
    assert_exact(&cap, "broadcast");
    assert_eq!(cap.connections.len(), 3);
    assert_eq!(kinds(&cap, 2), vec![100, 101, 105, 118, 154, 155]);
    assert!(
        kinds(&cap, 0).contains(&155),
        "the guest gets the broadcast"
    );
    assert!(kinds(&cap, 1).contains(&155), "the other admin gets it");
    let msgs = messages(&cap, 2);
    let Message::SendBroadcast(b) = msgs[4] else {
        panic!("{:?}", msgs[4].kind())
    };
    assert_eq!(b.text, b"hello all");
    let Message::RcvBroadcast(r) = msgs[5] else {
        panic!("{:?}", msgs[5].kind())
    };
    assert_eq!(r.text, b"hello all");
    assert_eq!(
        r.user_id,
        cap_user_id(&cap, 2),
        "the sender gets its own 155"
    );
}

fn cap_user_id(cap: &Capture, conn: usize) -> u32 {
    messages(cap, conn)
        .iter()
        .find_map(|m| match m {
            Message::ServerInfo(i) => Some(i.user_id),
            _ => None,
        })
        .expect("105")
}

#[test]
fn disconnect_ban_is_byte_exact() {
    let cap = load("disconnect-ban");
    assert_clean(&cap, "disconnect-ban");
    assert_exact(&cap, "disconnect-ban");
    assert_eq!(cap.connections.len(), 5);
    // The wrong password: the server's proof has status Failed (1).
    let failed = cap.connections[0]
        .records
        .iter()
        .find_map(|r| match r {
            Record::Proof {
                dir: Dir::ServerToClient,
                proof,
                ..
            } => Some(proof.status),
            _ => None,
        })
        .unwrap();
    assert_eq!(failed, openkdx_proto::handshake::ProofStatus::Failed);
    assert!(kinds(&cap, 0).is_empty(), "nothing after a failed login");
    // The victim: 151 with the banned flag, then the close.
    assert_eq!(kinds(&cap, 1), vec![100, 101, 105, 118, 103, 104, 103, 151]);
    let v = messages(&cap, 1);
    let Message::DisconnectMessage(d) = v[7] else {
        panic!("{:?}", v[7].kind())
    };
    assert_eq!(d.message, b"bye");
    assert_eq!(d.flags & 2, 2);
    // The admin: 150 → success Status, then the victim's 104.
    assert_eq!(kinds(&cap, 3), vec![100, 101, 105, 118, 150, 1, 104]);
    let a = messages(&cap, 3);
    let Message::DisconnectUser(du) = a[4] else {
        panic!("{:?}", a[4].kind())
    };
    assert_eq!((du.user_id, du.ban_seconds), (cap_user_id(&cap, 1), 10));
    // The server's ban list does not apply to 127.0.0.1: the next guest
    // login succeeds.
    assert_eq!(kinds(&cap, 4), vec![100, 101, 105, 118]);
}

#[test]
fn server_history_is_byte_exact() {
    let cap = load("server-history");
    assert_clean(&cap, "server-history");
    assert_exact(&cap, "server-history");
    assert_eq!(
        kinds(&cap, 0),
        vec![
            100, 101, 105, 118, 180, 181, 103, 183, 183, 104, 103, 183, 183, 182
        ]
    );
    let msgs = messages(&cap, 0);
    let Message::HistoryList(list) = msgs[5] else {
        panic!("{:?}", msgs[5].kind())
    };
    assert!(list.records.iter().any(|r| r.kind == HistoryKind::LOGIN));
    let Message::HistoryAdded(h) = msgs[7] else {
        panic!("{:?}", msgs[7].kind())
    };
    assert_eq!(h.record.kind, HistoryKind::LOGIN);
    assert_eq!(h.record.nick, b"passer-by");
    // A guest's 611 gets an error Status: no Post News privilege.
    let g = messages(&cap, 2);
    let Message::Status(s) = g[5] else {
        panic!("{:?}", g[5].kind())
    };
    assert!(s.is_error());
    assert_eq!(s.request_type_or_value, 611);
}

#[test]
fn connection_monitor_is_byte_exact() {
    let cap = load("connection-monitor");
    assert_clean(&cap, "connection-monitor");
    assert_exact(&cap, "connection-monitor");
    assert_eq!(cap.connections.len(), 3);
    assert_eq!(kinds(&cap, 1), vec![412, 413, 414, 414]);
    assert_eq!(
        kinds(&cap, 2),
        vec![
            100, 101, 105, 118, 140, 142, 143, 142, 143, 142, 143, 142, 143, 141
        ]
    );
    let msgs = messages(&cap, 2);
    let Message::ConnectionMonitorInfo(info) = msgs[6] else {
        panic!("{:?}", msgs[6].kind())
    };
    let transfer = info
        .records
        .iter()
        .find(|r| r.is_transfer())
        .expect("the download shows as a transfer");
    assert_eq!(transfer.bytes_total, 4 * 1024 * 1024);
    assert_eq!(transfer.path.to_string(), "four.bin");
    assert!(
        info.records
            .iter()
            .any(|r| !r.is_transfer() && r.nick == b"downloader")
    );
}

#[test]
fn a_cut_transfer_is_data_cut_only_when_the_stream_closed() {
    use openkdx_dissect::capture::{parse_packets, read_capture, tcp_flows};
    use openkdx_dissect::connection::dissect_flow;
    let path = capture_path("upload-resume");
    let cap = load("upload-resume");
    let keys = cap.connections[0].harvested_keys.clone();
    let frames = read_capture(&path).unwrap();
    let packets = parse_packets(&frames);
    let mut flows = tcp_flows(&packets, |p| p == 10700);
    flows.sort_by_key(|f| f.first_frame);
    let mut flow = flows[1].clone();
    assert!(flow.c2s.closed, "the client closed the cut transfer");
    let o = opts();
    let with_close = dissect_flow(1, &flow, &o, &keys);
    assert!(
        with_close
            .records
            .iter()
            .any(|r| matches!(r, Record::DataCut { .. }))
    );
    // The same bytes with no FIN/RST seen: the capture stopped early, so it
    // is a stop, not a cut.
    flow.c2s.closed = false;
    flow.s2c.closed = false;
    let truncated = dissect_flow(1, &flow, &o, &keys);
    assert!(
        truncated
            .records
            .iter()
            .any(|r| matches!(r, Record::Stopped { .. }))
    );
    assert!(
        !truncated
            .records
            .iter()
            .any(|r| matches!(r, Record::DataCut { .. }))
    );
}
