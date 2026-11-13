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

async fn connect(server: &MockServer) -> Connection {
    let mut stream = TcpStream::connect(server.addr).await.unwrap();
    let out = client_handshake(&mut stream, b"admin", &key("pw"), &table())
        .await
        .unwrap();
    Connection::spawn(stream, out.read_buf, &key("pw"), CLIENT_RECV_CAP)
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
    assert!(matches!(
        events.recv().await.unwrap(),
        Event::Disconnected(None)
    ));
    assert!(conn.is_closed());
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
}
