mod common;

use std::time::Duration;

use common::mock::{MockServer, Step, key, table};
use openkdx_client::Identity;
use openkdx_client::session::{ConnectOptions, Credentials, Session};
use openkdx_proto::msg::chat::MustJoinChat;
use openkdx_proto::msg::common::{Color, UserRecord};
use openkdx_proto::msg::session::{ServerGreeting, ServerInfo, UserDeparted, UserJoined, UserList};
use openkdx_proto::{Message, MessageType};

fn user(id: u32, nick: &str) -> UserRecord {
    UserRecord {
        user_id: id,
        status: 0,
        ignored_05: [0; 5],
        client_port: 10600,
        text_color: Color { r: 0, g: 0, b: 0 },
        background_color: Color { r: 0, g: 0, b: 0 },
        account_color: Color { r: 0, g: 0, b: 0 },
        flags: 0,
        ignored_1b: 0,
        icon: Vec::new(),
        extra: Vec::new(),
        nick: nick.as_bytes().to_vec(),
        info: Vec::new(),
        padding: Vec::new(),
    }
}

fn server_info(user_id: u32) -> ServerInfo {
    ServerInfo {
        session_key: [0xab; 32],
        file_transfer_port: 0,
        user_id,
        version: 0x0654,
        can_modify_settings: 1,
        ignored_2c: [0; 26],
        value_46: 2,
        ignored_4b: 0,
        name: b"New Server".to_vec(),
        description: Vec::new(),
        address: Vec::new(),
    }
}

fn creds() -> Credentials {
    Credentials {
        login: b"admin".to_vec(),
        password: b"pw".to_vec(),
    }
}

fn opts() -> ConnectOptions {
    ConnectOptions {
        identity: Identity::default(),
        timeout: Duration::from_secs(5),
        table: Some(table()),
    }
}

fn accounts() -> Vec<(Vec<u8>, openkdx_proto::AccountKey)> {
    vec![(b"admin".to_vec(), key("pw")), (Vec::new(), key(""))]
}

#[tokio::test]
async fn login_collects_user_list_and_server_info() {
    let server = MockServer::start(
        accounts(),
        vec![vec![
            Step::Expect(MessageType::SET_IDENTITY),
            Step::Expect(MessageType::LICENSE_REPORT),
            Step::Send(Message::UserList(UserList {
                users: vec![user(1, "openkdx")],
            })),
            Step::Send(Message::ServerInfo(server_info(1))),
            Step::Send(Message::ServerGreeting(ServerGreeting {
                ignored: [0; 4],
                text: b"Welcome".to_vec(),
            })),
        ]],
    )
    .await;
    let session = Session::connect(server.addr, &creds(), opts())
        .await
        .unwrap();
    assert_eq!(session.user_id(), 1);
    assert_eq!(session.server.name, b"New Server");
    assert_eq!(session.users().len(), 1);
    assert_eq!(session.session_key().as_bytes(), &[0xab; 32]);
    assert_eq!(session.transfer_addr(), server.addr);
    // The greeting may arrive after 105; give the reader a moment.
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(session.greeting(), Some(b"Welcome".to_vec()));
    assert_eq!(
        server.payloads(MessageType::LICENSE_REPORT),
        vec![vec![0, 0, 0, 1]]
    );
    server.check();
}

#[tokio::test]
async fn guest_login_sends_no_118_when_unregistered() {
    let server = MockServer::start(
        accounts(),
        vec![vec![
            Step::Expect(MessageType::SET_IDENTITY),
            Step::Send(Message::UserList(UserList {
                users: vec![user(3, "openkdx")],
            })),
            Step::Send(Message::ServerInfo(server_info(3))),
            Step::Expect(MessageType::PING),
        ]],
    )
    .await;
    let mut o = opts();
    o.identity.registered = false;
    o.timeout = Duration::from_millis(500);
    let session = Session::connect(server.addr, &Credentials::guest(), o)
        .await
        .unwrap();
    // A stray 118 would hit the mock's Expect(PING), which would panic and
    // drop the connection: the ping would then see Disconnected, not Timeout.
    let err = session.ping().await.unwrap_err();
    assert!(
        matches!(err, openkdx_client::ClientError::Timeout),
        "{err:?}"
    );
    assert!(server.payloads(MessageType::LICENSE_REPORT).is_empty());
    server.check();
}

#[tokio::test]
async fn user_list_tracks_joins_and_departs() {
    let server = MockServer::start(
        accounts(),
        vec![vec![
            Step::Expect(MessageType::SET_IDENTITY),
            Step::Expect(MessageType::LICENSE_REPORT),
            Step::Send(Message::UserList(UserList {
                users: vec![user(1, "openkdx")],
            })),
            Step::Send(Message::ServerInfo(server_info(1))),
            Step::Send(Message::UserJoined(UserJoined {
                user: user(2, "bob"),
            })),
            Step::Sleep(Duration::from_millis(100)),
            Step::Send(Message::UserDeparted(UserDeparted {
                user_id: 1,
                kind: 7,
                ignored: [0; 3],
            })),
        ]],
    )
    .await;
    let session = Session::connect(server.addr, &creds(), opts())
        .await
        .unwrap();
    wait_for(|| session.users().len() == 2).await;
    wait_for(|| session.users().len() == 1).await;
    assert_eq!(session.users()[0].nick, b"bob");
    server.check();
}

async fn wait_for(mut cond: impl FnMut() -> bool) {
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    while !cond() {
        assert!(
            std::time::Instant::now() < deadline,
            "condition not met in time"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn disconnect_during_login_carries_the_151() {
    use openkdx_proto::msg::admin::DisconnectMessage;
    let server = MockServer::start(
        accounts(),
        vec![vec![
            Step::Expect(MessageType::SET_IDENTITY),
            Step::Expect(MessageType::LICENSE_REPORT),
            Step::Send(Message::DisconnectMessage(DisconnectMessage {
                user_id: 0,
                ignored: [0; 8],
                flags: DisconnectMessage::FLAG_BANNED,
                value_0e: 0,
                value_10: 0,
                message: b"banned".to_vec(),
                name: Vec::new(),
            })),
            Step::Close,
        ]],
    )
    .await;
    match Session::connect(server.addr, &creds(), opts())
        .await
        .unwrap_err()
    {
        openkdx_client::ClientError::DisconnectedBy(d) => assert_eq!(d.message, b"banned"),
        other => panic!("{other:?}"),
    }
    server.check();
}

#[tokio::test]
async fn transfer_port_wider_than_u16_falls_back_to_main_port() {
    let server = MockServer::start(
        accounts(),
        vec![vec![
            Step::Expect(MessageType::SET_IDENTITY),
            Step::Expect(MessageType::LICENSE_REPORT),
            Step::Send(Message::UserList(UserList {
                users: vec![user(1, "openkdx")],
            })),
            Step::Send(Message::ServerInfo(ServerInfo {
                file_transfer_port: 0x1_0001,
                ..server_info(1)
            })),
        ]],
    )
    .await;
    let session = Session::connect(server.addr, &creds(), opts())
        .await
        .unwrap();
    assert_eq!(session.transfer_addr(), server.addr);
    server.check();
}

#[tokio::test]
async fn must_join_chat_is_recorded() {
    let server = MockServer::start(
        accounts(),
        vec![vec![
            Step::Expect(MessageType::SET_IDENTITY),
            Step::Expect(MessageType::LICENSE_REPORT),
            Step::Send(Message::UserList(UserList {
                users: vec![user(1, "openkdx")],
            })),
            Step::Send(Message::MustJoinChat(MustJoinChat)),
            Step::Send(Message::ServerInfo(server_info(1))),
        ]],
    )
    .await;
    let session = Session::connect(server.addr, &creds(), opts())
        .await
        .unwrap();
    assert!(session.must_join_chat());
    server.check();
}
