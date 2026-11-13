mod common;

use common::fixture::session_with;
use common::mock::Step;
use openkdx_client::admin::set_server_name;
use openkdx_proto::msg::Status;
use openkdx_proto::msg::admin::{
    ConnectionMonitorInfo, HistoryAdded, HistoryKind, HistoryList, HistoryRecord,
    MonitoredConnection, ServerSettings, ServerSettingsInfo, ServerSettingsMain,
};
use openkdx_proto::{Message, MessageType, Payload};

fn settings() -> ServerSettings {
    let main = ServerSettingsMain {
        ignored_00: [0; 35],
        flags: 2,
        ignored_24: [0; 20],
        total_outgoing_speed_limit: 0,
        ignored_3c: [0; 22],
        ignored_52: 10,
        ignored_53: [0; 9],
        server_name: b"New Server".to_vec(),
        name_tail: vec![0; 21],
    }
    .to_bytes();
    let other = vec![0xee; 7];
    let mut table = [(0u32, 0u32); 11];
    table[0] = (ServerSettings::HEADER_LEN as u32, main.len() as u32);
    table[1] = (
        (ServerSettings::HEADER_LEN + main.len()) as u32,
        other.len() as u32,
    );
    let mut body = main;
    body.extend(other);
    ServerSettings {
        version: 1,
        ignored: [0; 2],
        table,
        value_64: [1, 1, 1, 1, 0, 1, 0, 2, 0, 0, 0, 0],
        body,
    }
}

#[test]
fn set_server_name_rewrites_only_section_0_and_keeps_its_length() {
    let before = settings();
    let mut s = before.clone();
    set_server_name(&mut s, b"OpenKDX Lab").unwrap();
    assert_eq!(s.main().unwrap().server_name, b"OpenKDX Lab");
    assert_eq!(
        s.main().unwrap().ignored_52,
        11,
        "the length byte at 0x52 follows the name"
    );
    assert_eq!(s.table, before.table);
    assert_eq!(s.section(1), before.section(1));
    assert_eq!(s.body.len(), before.body.len());
    assert!(s.main().unwrap().greeting_enabled());
    let mut back = s.clone();
    set_server_name(&mut back, b"New Server").unwrap();
    assert_eq!(back, before);
}

#[test]
fn set_server_name_rejects_a_name_that_does_not_fit() {
    let mut s = settings();
    assert!(set_server_name(&mut s, &[b'x'; 40]).is_err());
    assert_eq!(s, settings());
}

#[tokio::test]
async fn server_settings_round_trip_and_set() {
    let (server, session) = session_with(vec![
        Step::Expect(MessageType::GET_SERVER_SETTINGS),
        Step::Reply(Message::ServerSettingsInfo(ServerSettingsInfo {
            settings: settings(),
        })),
        Step::Expect(MessageType::SET_SERVER_SETTINGS),
        Step::Reply(Message::Status(Status {
            code: 0,
            request_type_or_value: 172,
            detail: None,
        })),
    ])
    .await;
    let mut s = session.server_settings().await.unwrap();
    assert_eq!(s.main().unwrap().server_name, b"New Server");
    set_server_name(&mut s, b"Lab").unwrap();
    session.set_server_settings(s).await.unwrap();
    let sent = openkdx_proto::msg::admin::SetServerSettings::decode(
        &server.payloads(MessageType::SET_SERVER_SETTINGS)[0],
    )
    .unwrap();
    assert_eq!(sent.settings.main().unwrap().server_name, b"Lab");
    server.check();
}

#[tokio::test]
async fn history_join_returns_list_then_events() {
    let rec = HistoryRecord {
        kind: HistoryKind::LOGIN,
        user_id: 2,
        nick: b"bob".to_vec(),
        login: Vec::new(),
        padding: vec![0; 3],
        ..HistoryRecord::default()
    };
    let (server, session) = session_with(vec![
        Step::Expect(MessageType::SERVER_HISTORY_JOIN),
        Step::Reply(Message::HistoryList(HistoryList {
            ignored: 0,
            records: vec![rec.clone()],
        })),
        Step::Send(Message::HistoryAdded(HistoryAdded {
            record: rec.clone(),
        })),
        Step::Expect(MessageType::SERVER_HISTORY_LEAVE),
    ])
    .await;
    let mut events = session.events();
    let list = session.history_join().await.unwrap();
    assert_eq!(list.records[0].nick, b"bob");
    let added = session
        .wait_for(&mut events, |m| match m {
            Message::HistoryAdded(h) => Some(h.record.clone()),
            _ => None,
        })
        .await
        .unwrap();
    assert_eq!(added.kind, HistoryKind::LOGIN);
    session.history_leave().await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    server.check();
}

#[tokio::test]
async fn monitor_info_round_trip_and_disconnect_user() {
    use openkdx_proto::msg::admin::DisconnectUser;
    let (server, session) = session_with(vec![
        Step::Expect(MessageType::OPEN_CONNECTION_MONITOR),
        Step::Expect(MessageType::GET_CONNECTION_MONITOR_INFO),
        Step::Reply(Message::ConnectionMonitorInfo(ConnectionMonitorInfo {
            records: vec![MonitoredConnection {
                user_id: 2,
                transfer_id: 0,
                nick: b"bob".to_vec(),
                padding: vec![0; 2],
                ..MonitoredConnection::default()
            }],
        })),
        Step::Expect(MessageType::CLOSE_CONNECTION_MONITOR),
        Step::Expect(MessageType::DISCONNECT_USER),
        Step::Expect(MessageType::SEND_BROADCAST_MESSAGE),
    ])
    .await;
    session.monitor_open().await.unwrap();
    let info = session.monitor_info().await.unwrap();
    assert_eq!(info.records[0].nick, b"bob");
    session.monitor_close().await.unwrap();
    session.disconnect_user(2, 10, b"bye", true).await.unwrap();
    session.broadcast(b"hello all").await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    let d = DisconnectUser::decode(&server.payloads(MessageType::DISCONNECT_USER)[0]).unwrap();
    assert_eq!(
        (
            d.user_id,
            d.ban_seconds,
            d.options & 1,
            d.message.as_slice()
        ),
        (2, 10, 1, &b"bye"[..])
    );
    server.check();
}
