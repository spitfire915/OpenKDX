mod common;

use std::time::Duration;

use common::mock::{MockServer, Step, key, table};
use openkdx_client::ClientError;
use openkdx_client::session::{ConnectOptions, Credentials, Session};
use openkdx_proto::msg::chat::{ChatWindowInfo, SendChatText};
use openkdx_proto::msg::common::{Color, Time1900, UserRecord};
use openkdx_proto::msg::session::{ServerInfo, UserList};
use openkdx_proto::{Message, MessageType, Payload};

fn user(id: u32) -> UserRecord {
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
        nick: b"openkdx".to_vec(),
        info: Vec::new(),
        padding: Vec::new(),
    }
}

fn login_steps() -> Vec<Step> {
    vec![
        Step::Expect(MessageType::SET_IDENTITY),
        Step::Expect(MessageType::LICENSE_REPORT),
        Step::Send(Message::UserList(UserList {
            users: vec![user(1)],
        })),
        Step::Send(Message::ServerInfo(ServerInfo {
            session_key: [0; 32],
            file_transfer_port: 0,
            user_id: 1,
            version: 0x0654,
            can_modify_settings: 1,
            ignored_2c: [0; 26],
            value_46: 2,
            ignored_4b: 0,
            name: b"New Server".to_vec(),
            description: Vec::new(),
            address: Vec::new(),
        })),
    ]
}

async fn session_with(steps: Vec<Step>) -> (MockServer, Session) {
    let mut script = login_steps();
    script.extend(steps);
    let server = MockServer::start(vec![(b"admin".to_vec(), key("pw"))], vec![script]).await;
    let session = Session::connect(
        server.addr,
        &Credentials {
            login: b"admin".to_vec(),
            password: b"pw".to_vec(),
        },
        ConnectOptions {
            timeout: Duration::from_secs(5),
            table: Some(table()),
            ..ConnectOptions::default()
        },
    )
    .await
    .unwrap();
    (server, session)
}

#[tokio::test]
async fn say_sets_action_flag_and_strips_nothing() {
    let (server, session) = session_with(vec![
        Step::Expect(MessageType::SEND_CHAT_TEXT),
        Step::Expect(MessageType::SEND_CHAT_TEXT),
    ])
    .await;
    session.say(1, b"hello", false).await.unwrap();
    session.say(1, b"waves", true).await.unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;
    let sent: Vec<SendChatText> = server
        .payloads(MessageType::SEND_CHAT_TEXT)
        .iter()
        .map(|p| SendChatText::decode(p).unwrap())
        .collect();
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[0].flags, 0);
    assert_eq!(sent[0].text, b"hello");
    assert_eq!(sent[0].user_id, 0);
    assert_eq!(sent[1].flags, SendChatText::FLAG_ACTION);
    assert_eq!(sent[1].text, b"waves");
    server.check();
}

#[tokio::test]
async fn say_rejects_text_over_the_limit() {
    let (server, session) = session_with(vec![]).await;
    let err = session
        .say(1, &vec![b'x'; SendChatText::MAX_TEXT + 1], false)
        .await
        .unwrap_err();
    assert!(matches!(err, ClientError::Protocol(_)), "{err:?}");
    server.check();
}

#[tokio::test]
async fn join_returns_window_info() {
    let info = ChatWindowInfo {
        time: Time1900(0),
        chat_id: 1,
        key: 0,
        ignored_10: [0; 4],
        flags: 0,
        ignored_18: [0; 6],
        ignored_23: 0,
        members: vec![1, 2],
        name: b"Public Chat".to_vec(),
        topic: b"hi".to_vec(),
        topic_setter: Vec::new(),
        irc_server: Vec::new(),
        irc_channel: Vec::new(),
    };
    let (server, session) = session_with(vec![
        Step::Expect(MessageType::JOIN_CHAT),
        Step::Reply(Message::ChatWindowInfo(info.clone())),
    ])
    .await;
    let got = session.join_chat(1, 0).await.unwrap();
    assert_eq!(got, info);
    server.check();
}

#[tokio::test]
async fn send_pm_rejects_empty() {
    let (server, session) = session_with(vec![]).await;
    let err = session.send_pm(2, b"").await.unwrap_err();
    assert!(matches!(err, ClientError::Protocol(_)), "{err:?}");
    server.check();
}

#[tokio::test]
async fn chat_list_round_trip() {
    use openkdx_proto::msg::chat::{ChatList, ChatListEntry};
    let (server, session) = session_with(vec![
        Step::Expect(MessageType::GET_CHAT_LIST),
        Step::Reply(Message::ChatList(ChatList {
            chats: vec![ChatListEntry {
                chat_id: 1,
                key: 0,
                ignored: [0; 6],
                flags: 0,
                member_count: 1,
                name: b"Public Chat".to_vec(),
                description: Vec::new(),
                padding: vec![0; 3],
            }],
        })),
    ])
    .await;
    let list = session.chat_list().await.unwrap();
    assert_eq!(list[0].chat_id, 1);
    server.check();
}
