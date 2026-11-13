mod common;

use std::time::Duration;

use common::mock::{MockServer, Step, key, table};
use openkdx_client::ClientError;
use openkdx_client::connection::{Connection, Event};
use openkdx_client::handshake::client_handshake;
use openkdx_proto::msg::chat::{ChatList, ChatListEntry, GetChatList};
use openkdx_proto::msg::session::UserDeparted;
use openkdx_proto::msg::{Status, StatusDetail};
use openkdx_proto::session::CLIENT_RECV_CAP;
use openkdx_proto::{Message, MessageType};
use tokio::net::TcpStream;

async fn connect2(server: &MockServer) -> (Connection, tokio::sync::broadcast::Receiver<Event>) {
    let mut stream = TcpStream::connect(server.addr).await.unwrap();
    let out = client_handshake(&mut stream, b"admin", &key("pw"), &table())
        .await
        .unwrap();
    Connection::spawn(stream, out.read_buf, &key("pw"), CLIENT_RECV_CAP)
}

async fn connect(server: &MockServer) -> Connection {
    connect2(server).await.0
}

fn accounts() -> Vec<(Vec<u8>, openkdx_proto::AccountKey)> {
    vec![(b"admin".to_vec(), key("pw"))]
}

fn chat_list() -> ChatList {
    ChatList {
        chats: vec![ChatListEntry {
            chat_id: 1,
            key: 0,
            ignored: [0; 6],
            flags: 0,
            member_count: 2,
            name: b"Public Chat".to_vec(),
            description: Vec::new(),
            padding: vec![0; 3],
        }],
    }
}

#[tokio::test]
async fn request_gets_reply_by_txid() {
    let departed = UserDeparted {
        user_id: 9,
        kind: 7,
        ignored: [0; 3],
    };
    let server = MockServer::start(
        accounts(),
        vec![vec![
            Step::Expect(MessageType::GET_CHAT_LIST),
            Step::Send(Message::UserDeparted(departed)),
            Step::Reply(Message::ChatList(chat_list())),
        ]],
    )
    .await;
    let conn = connect(&server).await;
    let mut events = conn.events();
    let list: ChatList = conn.request_as(&GetChatList).await.unwrap();
    assert_eq!(list.chats[0].name, b"Public Chat");
    match events.recv().await.unwrap() {
        Event::Message(m) => match *m {
            Message::UserDeparted(d) => assert_eq!(d, departed),
            other => panic!("{other:?}"),
        },
        other => panic!("{other:?}"),
    }
    server.check();
}

#[tokio::test]
async fn status_reply_becomes_server_error() {
    let server = MockServer::start(
        accounts(),
        vec![vec![
            Step::Expect(MessageType::GET_CHAT_LIST),
            Step::Reply(Message::Status(Status {
                code: 0x40070,
                request_type_or_value: 214,
                detail: Some(StatusDetail {
                    module: 0x54524e,
                    line: 1,
                    extra: 0,
                    ignored: 0,
                }),
            })),
        ]],
    )
    .await;
    let conn = connect(&server).await;
    let err = conn
        .request_as::<_, ChatList>(&GetChatList)
        .await
        .unwrap_err();
    match err {
        ClientError::Server {
            code,
            name,
            request,
        } => {
            assert_eq!(code, 0x40070);
            assert!(name.is_some());
            assert_eq!(request, 214);
        }
        other => panic!("{other:?}"),
    }
    server.check();
}

#[tokio::test]
async fn pending_requests_fail_when_peer_closes() {
    let server = MockServer::start(
        accounts(),
        vec![vec![Step::Expect(MessageType::GET_CHAT_LIST), Step::Close]],
    )
    .await;
    let conn = connect(&server).await;
    let mut events = conn.events();
    let err = conn.request(&GetChatList).await.unwrap_err();
    assert!(matches!(err, ClientError::Disconnected), "{err:?}");
    match events.recv().await.unwrap() {
        Event::Disconnected(d) => assert!(d.message.is_none() && d.error.is_none(), "{d:?}"),
        other => panic!("{other:?}"),
    }
    assert!(conn.is_closed());
    server.check();
}

#[tokio::test]
async fn request_times_out() {
    let server = MockServer::start(
        accounts(),
        vec![vec![
            Step::Expect(MessageType::GET_CHAT_LIST),
            Step::Sleep(Duration::from_secs(2)),
            Step::Reply(Message::ChatList(chat_list())),
        ]],
    )
    .await;
    let conn = connect(&server)
        .await
        .with_timeout(Duration::from_millis(200));
    let err = conn.request(&GetChatList).await.unwrap_err();
    assert!(matches!(err, ClientError::Timeout), "{err:?}");
    server.check();
}

#[tokio::test]
async fn close_shuts_down_and_reports_closed() {
    let server = MockServer::start(accounts(), vec![vec![]]).await;
    let (conn, mut events) = connect2(&server).await;
    conn.close().await;
    assert!(conn.is_closed());
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(2), events.recv())
            .await
            .unwrap()
            .unwrap(),
        Event::Disconnected(_)
    ));
    assert!(matches!(
        conn.send(&GetChatList).await.unwrap_err(),
        ClientError::Disconnected
    ));
    server.check();
}

#[tokio::test]
async fn send_after_peer_close_is_disconnected() {
    let server = MockServer::start(accounts(), vec![vec![Step::Close]]).await;
    let (conn, mut events) = connect2(&server).await;
    assert!(matches!(
        events.recv().await.unwrap(),
        Event::Disconnected(_)
    ));
    assert!(matches!(
        conn.send(&GetChatList).await.unwrap_err(),
        ClientError::Disconnected
    ));
    server.check();
}

#[tokio::test]
async fn data_on_main_connection_is_a_protocol_error() {
    let server = MockServer::start(
        accounts(),
        vec![vec![Step::DataReply {
            kind: MessageType::DOWNLOAD_FILE_DATA,
            reply: false,
            bytes: vec![1, 2, 3],
        }]],
    )
    .await;
    let (_conn, mut events) = connect2(&server).await;
    match events.recv().await.unwrap() {
        Event::Disconnected(d) => assert!(d.error.is_some(), "{d:?}"),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn disconnect_message_is_carried_to_the_waiter() {
    use openkdx_proto::msg::admin::DisconnectMessage;
    let server = MockServer::start(
        accounts(),
        vec![vec![
            Step::Expect(MessageType::GET_CHAT_LIST),
            Step::Send(Message::DisconnectMessage(DisconnectMessage {
                user_id: 1,
                ignored: [0; 8],
                flags: DisconnectMessage::FLAG_BANNED,
                value_0e: 0,
                value_10: 0,
                message: b"bye".to_vec(),
                name: b"admin".to_vec(),
            })),
            Step::Close,
        ]],
    )
    .await;
    let (conn, _events) = connect2(&server).await;
    match conn.request(&GetChatList).await.unwrap_err() {
        ClientError::DisconnectedBy(d) => assert_eq!(d.message, b"bye"),
        other => panic!("{other:?}"),
    }
    server.check();
}
