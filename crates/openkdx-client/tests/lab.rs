//! Differential tests: the client against KDX Server 1.620 (and Tracker
//! 1.110) in the lab. They run only with `OPENKDX_LAB_SERVER` set (see
//! `lab/differential.sh`), serially, and open at most two connections each:
//! Server 1.620 bans an address after 30 connections in 300 s.

use std::net::SocketAddr;
use std::time::Duration;

use openkdx_client::ClientError;
use openkdx_client::accounts::new_account;
use openkdx_client::admin::set_server_name;
use openkdx_client::connection::Event;
use openkdx_client::path::parse_path;
use openkdx_client::session::{ConnectOptions, Credentials, Session};
use openkdx_client::tracker::Tracker;
use openkdx_client::transfer::Transfer;
use openkdx_client::{Identity, load_kdx_table};
use openkdx_proto::Message;
use openkdx_proto::msg::accounts::{Privilege, Privileges};
use openkdx_proto::msg::admin::{DisconnectMessage, HistoryKind};
use openkdx_proto::msg::misc::{FolderAccess, FolderAccessRule};

struct Lab {
    server: SocketAddr,
    tracker: Option<SocketAddr>,
    password: Vec<u8>,
}

fn lab() -> Option<Lab> {
    let Ok(server) = std::env::var("OPENKDX_LAB_SERVER") else {
        eprintln!("skipped: OPENKDX_LAB_SERVER not set");
        return None;
    };
    Some(Lab {
        server: server.parse().expect("OPENKDX_LAB_SERVER is HOST:PORT"),
        tracker: std::env::var("OPENKDX_LAB_TRACKER")
            .ok()
            .map(|t| t.parse().expect("OPENKDX_LAB_TRACKER is HOST:PORT")),
        password: std::env::var("OPENKDX_LAB_PASSWORD")
            .unwrap_or_else(|_| "openkdx-lab".into())
            .into_bytes(),
    })
}

fn opts(nick: &str) -> ConnectOptions {
    ConnectOptions {
        identity: Identity {
            nick: nick.as_bytes().to_vec(),
            ..Identity::default()
        },
        timeout: Duration::from_secs(20),
        table: Some(load_kdx_table().expect("the KDX challenge table")),
    }
}

impl Lab {
    fn admin(&self) -> Credentials {
        Credentials {
            login: b"admin".to_vec(),
            password: self.password.clone(),
        }
    }

    async fn login_admin(&self, nick: &str) -> Session {
        Session::connect(self.server, &self.admin(), opts(nick))
            .await
            .expect("admin login")
    }

    async fn login_guest(&self, nick: &str) -> Session {
        Session::connect(self.server, &Credentials::guest(), opts(nick))
            .await
            .expect("guest login")
    }
}

macro_rules! lab_test {
    ($name:ident, |$lab:ident| $body:block) => {
        #[tokio::test]
        async fn $name() {
            let Some($lab) = lab() else { return };
            $body
        }
    };
}

lab_test!(lab_login_admin, |lab| {
    let s = lab.login_admin("admin-login").await;
    assert_eq!(s.server.version, 0x0654, "Server 1.620");
    assert!(s.user_id() >= 1);
    assert_eq!(s.server.can_modify_settings, 1);
    assert!(s.users().iter().any(|u| u.user_id == s.user_id()));
    s.ping().await.unwrap();
    s.close().await;
});

lab_test!(lab_login_guest, |lab| {
    let s = lab.login_guest("guest-login").await;
    assert_eq!(s.server.can_modify_settings, 0);
    s.close().await;
});

lab_test!(lab_login_wrong_password, |lab| {
    let err = Session::connect(
        lab.server,
        &Credentials {
            login: b"admin".to_vec(),
            password: b"not-the-password".to_vec(),
        },
        opts("x"),
    )
    .await
    .unwrap_err();
    assert!(
        matches!(
            err,
            ClientError::LoginFailed(_) | ClientError::ProofMismatch
        ),
        "{err:?}"
    );
});

lab_test!(lab_chat_say_and_echo, |lab| {
    let s = lab.login_admin("chatter").await;
    let list = s.chat_list().await.unwrap();
    let public = list.iter().find(|c| c.chat_id == 1).expect("Public Chat");
    let info = s.join_chat(public.chat_id, public.key).await.unwrap();
    assert!(info.members.contains(&s.user_id()));
    let mut events = s.events();
    s.say(1, b"hello from openkdx", false).await.unwrap();
    let echo = s
        .wait_for(&mut events, |m| match m {
            Message::ReceiveChatText(t) if t.text == b"hello from openkdx" => Some(t.clone()),
            _ => None,
        })
        .await
        .unwrap();
    assert_eq!(echo.user_id, s.user_id());
    assert!(!echo.is_action());
    s.say(1, b"waves", true).await.unwrap();
    let action = s
        .wait_for(&mut events, |m| match m {
            Message::ReceiveChatText(t) if t.text == b"waves" => Some(t.clone()),
            _ => None,
        })
        .await
        .unwrap();
    assert!(action.is_action(), "the /me flag survives the round trip");
    s.set_topic(1, b"openkdx was here").await.unwrap();
    s.wait_for(&mut events, |m| match m {
        Message::ChatTopicChanged(t) if t.topic == b"openkdx was here" => Some(()),
        _ => None,
    })
    .await
    .unwrap();
    s.close().await;
});

lab_test!(lab_pm_between_two_sessions, |lab| {
    let a = lab.login_admin("pm-a").await;
    let b = lab.login_guest("pm-b").await;
    let mut events = b.events();
    a.send_pm(b.user_id(), b"psst").await.unwrap();
    let pm = b
        .wait_for(&mut events, |m| match m {
            Message::IncomingPrivateMessage(p) => Some(p.clone()),
            _ => None,
        })
        .await
        .unwrap();
    assert_eq!(pm.user_id, a.user_id());
    assert_eq!(pm.text, b"psst");
    a.close().await;
    b.close().await;
});

lab_test!(lab_news_post_and_read, |lab| {
    let s = lab.login_admin("newsie").await;
    let groups = s.newsgroups().await.unwrap();
    let group = groups.first().expect("a newsgroup").group_id;
    let id = s
        .post_news(group, b"from openkdx", b"line one\nline two")
        .await
        .unwrap();
    let msg = s.news_message(group, id).await.unwrap();
    assert_eq!(msg.subject, b"from openkdx");
    assert_eq!(msg.body, b"line one\rline two");
    let list = s.news_messages(group).await.unwrap();
    assert!(list.messages.iter().any(|m| m.message_id == id));
    s.close().await;
});

fn body(n: usize) -> Vec<u8> {
    let mut x: u32 = 0x9e37_79b9;
    (0..n)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 17;
            x ^= x << 5;
            x as u8
        })
        .collect()
}

lab_test!(lab_files_list_get_put_resume, |lab| {
    let s = lab.login_admin("filer").await;
    let root = s.list_files(&parse_path("")).await.unwrap();
    let uploads = root
        .iter()
        .find(|e| e.is_folder() && e.access_item == 0x2b)
        .expect("an [UL] folder");
    let folder = parse_path(&String::from_utf8_lossy(&uploads.name));
    let data = body(256 * 1024);
    let dir = tempfile::tempdir().unwrap();
    let local = dir.path().join("big.bin");
    std::fs::write(&local, &data).unwrap();
    let name = format!("openkdx-{}.bin", std::process::id());

    // A partial upload: stop after 150000 bytes. The server keeps the
    // partial as `<name>.prt` and flushes it in large chunks, so what it
    // holds is some multiple of 0x1000 below what was sent.
    let mut t = Transfer::open(&s).await.unwrap();
    let err = t
        .upload(
            &local,
            &folder,
            name.as_bytes(),
            false,
            |_| {},
            Some(150_000),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ClientError::Transfer(_)), "{err:?}");
    drop(t);
    tokio::time::sleep(Duration::from_millis(500)).await;
    let listed = s.list_files(&folder).await.unwrap();
    assert!(
        listed
            .iter()
            .any(|e| e.name == format!("{name}.prt").as_bytes()),
        "the partial is listed as .prt: {:?}",
        listed
            .iter()
            .map(|e| String::from_utf8_lossy(&e.name).into_owned())
            .collect::<Vec<_>>()
    );

    // Resume from 416's offset.
    let mut t = Transfer::open(&s).await.unwrap();
    let mut first = None;
    t.upload(
        &local,
        &folder,
        name.as_bytes(),
        true,
        |p| {
            first.get_or_insert(p.done);
        },
        None,
    )
    .await
    .unwrap();
    let first = first.unwrap();
    assert!(
        first > 0 && first % 0x1000 == 0 && first <= 150_000,
        "416 data_offset {first:#x}"
    );

    let listed = s.list_files(&folder).await.unwrap();
    let entry = listed
        .iter()
        .find(|e| e.name == name.as_bytes())
        .expect("the upload is listed");
    assert_eq!(entry.size, data.len() as u64);

    // Download it whole, then resume a truncated copy.
    let remote = openkdx_client::path::join(&folder, name.as_bytes());
    let dest = dir.path().join("down.bin");
    let mut t = Transfer::open(&s).await.unwrap();
    let info = t.download(&remote, &dest, false, |_| {}).await.unwrap();
    assert_eq!(info.data_size, data.len() as u64);
    assert_eq!(std::fs::read(&dest).unwrap(), data);
    std::fs::write(&dest, &data[..5000]).unwrap();
    t.download(&remote, &dest, true, |_| {}).await.unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), data);
    t.close().await;
    s.close().await;
});

lab_test!(lab_accounts_create_show_delete, |lab| {
    let s = lab.login_admin("accountant").await;
    let login = format!("tester{}", std::process::id() % 1000);
    let privs = Privileges::from_bits(&[Privilege::CanLogin, Privilege::Download]);
    s.create_account(new_account(
        login.as_bytes(),
        b"Tester",
        Some(b"t"),
        1,
        privs,
        privs,
    ))
    .await
    .unwrap();
    let shown = s.account(login.as_bytes()).await.unwrap();
    assert_eq!(shown.name, b"Tester");
    assert!(shown.has_password());
    assert_eq!(shown.class_index, 1);
    assert!(
        s.accounts()
            .await
            .unwrap()
            .iter()
            .any(|a| a.login == login.as_bytes())
    );
    let mut changed = shown.clone();
    changed.key = [0; 32];
    changed.name = b"Tester 2".to_vec();
    s.set_account(changed).await.unwrap();
    assert_eq!(s.account(login.as_bytes()).await.unwrap().name, b"Tester 2");
    s.delete_account(login.as_bytes()).await.unwrap();
    assert!(
        !s.accounts()
            .await
            .unwrap()
            .iter()
            .any(|a| a.login == login.as_bytes())
    );
    s.close().await;
});

lab_test!(lab_classes_list_and_show, |lab| {
    let s = lab.login_admin("classy").await;
    let classes = s.classes().await.unwrap();
    assert!(!classes.is_empty());
    let first = s.class(classes[0].class_index).await.unwrap();
    assert_eq!(first.class_index, classes[0].class_index);
    s.close().await;
});

lab_test!(lab_folder_access_list_create_delete, |lab| {
    let s = lab.login_admin("fa").await;
    let before = s.folder_access_list().await.unwrap();
    assert!(before.iter().any(|i| i.icon == FolderAccess::ICON_UPLOADS));
    let name = format!("Pictures{}", std::process::id() % 1000);
    s.create_folder_access(FolderAccess {
        ignored_00: [0; 4],
        flags: 0,
        icon: FolderAccess::ICON_PICTURES,
        class_rules: [FolderAccessRule::default(); 32],
        rule: FolderAccessRule::default(),
        ignored_90: [0; 29],
        ignored_b0: 0,
        ignored_b2: [0; 6],
        name: name.clone().into_bytes(),
        owner: Vec::new(),
        description: b"pics".to_vec(),
        current_name: Vec::new(),
        padding: Vec::new(),
    })
    .await
    .unwrap();
    let shown = s.folder_access(name.as_bytes()).await.unwrap();
    assert_eq!(shown.description, b"pics");
    s.delete_folder_access(name.as_bytes()).await.unwrap();
    let after = s.folder_access_list().await.unwrap();
    assert!(!after.iter().any(|i| i.name == name.as_bytes()));
    s.close().await;
});

lab_test!(lab_server_settings_set_name_round_trip, |lab| {
    let s = lab.login_admin("settings").await;
    let original = s.server_settings().await.unwrap();
    let old_name = original.main.name.clone();
    let mut changed = original.clone();
    set_server_name(&mut changed, b"OpenKDX Lab").unwrap();
    s.set_server_settings(changed).await.unwrap();
    let again = s.server_settings().await.unwrap();
    assert_eq!(again.main.name, b"OpenKDX Lab");
    let mut restore = again;
    set_server_name(&mut restore, &old_name).unwrap();
    s.set_server_settings(restore).await.unwrap();
    s.close().await;
});

lab_test!(lab_broadcast_received_by_other_session, |lab| {
    let a = lab.login_admin("announcer").await;
    let b = lab.login_guest("listener").await;
    let mut events = b.events();
    a.broadcast(b"hello all").await.unwrap();
    let got = b
        .wait_for(&mut events, |m| match m {
            Message::RcvBroadcast(r) => Some(r.clone()),
            _ => None,
        })
        .await
        .unwrap();
    assert_eq!(got.text, b"hello all");
    a.close().await;
    b.close().await;
});

lab_test!(lab_disconnect_from_victim_side, |lab| {
    let admin = lab.login_admin("bouncer").await;
    let victim = lab.login_guest("victim").await;
    let mut events = victim.events();
    admin
        .disconnect_user(victim.user_id(), 5, b"bye", false)
        .await
        .unwrap();
    let mut saw_151 = None;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        match tokio::time::timeout_at(deadline, events.recv()).await {
            Ok(Ok(Event::Message(m))) => {
                if let Message::DisconnectMessage(d) = *m {
                    saw_151 = Some(d);
                }
            }
            Ok(Ok(Event::Disconnected(d))) => {
                let m: DisconnectMessage = d.message.or(saw_151).expect("a 151 before the close");
                assert_eq!(m.message, b"bye");
                assert!(m.flags & DisconnectMessage::FLAG_BANNED != 0);
                break;
            }
            other => panic!("expected the disconnect, got {other:?}"),
        }
    }
    assert!(victim.connection().is_closed());
    // Note: the ban is by address, and the admin shares 127.0.0.1, so a
    // reconnect test would ban the whole suite; `lab/scenarios/disconnect-ban.sh`
    // shows the ban itself.
    admin.close().await;
});

lab_test!(lab_history_join_sees_login_record, |lab| {
    let s = lab.login_admin("historian").await;
    let list = s.history_join().await.unwrap();
    assert!(
        list.records.iter().any(|r| r.kind == HistoryKind::LOGIN),
        "a Login record"
    );
    let mut events = s.events();
    let other = lab.login_guest("passer-by").await;
    let added = s
        .wait_for(&mut events, |m| match m {
            Message::HistoryAdded(h) if h.record.kind == HistoryKind::LOGIN => {
                Some(h.record.clone())
            }
            _ => None,
        })
        .await
        .unwrap();
    assert_eq!(added.user_id, other.user_id());
    s.history_leave().await.unwrap();
    other.close().await;
    s.close().await;
});

lab_test!(lab_connection_monitor_lists_sessions, |lab| {
    let s = lab.login_admin("monitor").await;
    s.monitor_open().await.unwrap();
    let info = s.monitor_info().await.unwrap();
    assert!(info.records.iter().any(|r| r.user_id == s.user_id()));
    s.monitor_close().await.unwrap();
    s.close().await;
});

lab_test!(lab_tracker_groups_and_servers, |lab| {
    let Some(addr) = lab.tracker else {
        eprintln!("skipped: OPENKDX_LAB_TRACKER not set");
        return;
    };
    let t = Tracker::connect(addr, &load_kdx_table().unwrap(), Duration::from_secs(20))
        .await
        .unwrap();
    let groups = t.groups().await.unwrap();
    assert!(!groups.is_empty());
    let servers = t.servers(&groups[0].name).await.unwrap();
    // A fresh lab server has no tracker configured (that takes the GUI's
    // Server Settings; see lab/scenarios/tracker-basic.sh), so the list is
    // usually empty here; `captures/tracker-basic` shows a registered one.
    if std::env::var("OPENKDX_LAB_TRACKER_HAS_SERVER").is_ok() {
        assert!(
            servers.iter().any(|s| s.port == lab.server.port()),
            "the lab server registered: {servers:?}"
        );
    }
    t.close().await;
});
