mod common;

use common::fixture::session_with;
use common::mock::Step;
use openkdx_client::ClientError;
use openkdx_proto::msg::news::{
    NewsMessage, NewsMessagePosted, NewsMessageReply, NewsMessages, NewsgroupEntry, NewsgroupList,
    PostNewsMessage,
};
use openkdx_proto::{Message, MessageType, Payload};

#[tokio::test]
async fn newsgroups_round_trip() {
    let (server, session) = session_with(vec![
        Step::Expect(MessageType::GET_NEWSGROUP_LIST),
        Step::Reply(Message::NewsgroupList(NewsgroupList {
            groups: vec![NewsgroupEntry {
                group_id: 1,
                message_count: 2,
                name: b"General News".to_vec(),
                padding: vec![0; 3],
                ..NewsgroupEntry::default()
            }],
        })),
    ])
    .await;
    let groups = session.newsgroups().await.unwrap();
    assert_eq!(groups[0].name, b"General News");
    server.check();
}

#[tokio::test]
async fn post_waits_for_613_and_returns_the_message_id() {
    let (server, session) = session_with(vec![
        Step::Expect(MessageType::POST_NEWS_MESSAGE),
        Step::Send(Message::NewsMessagePosted(NewsMessagePosted {
            group_id: 7,
            message_id: 0,
            message_count: 1,
        })),
        Step::Send(Message::NewsMessagePosted(NewsMessagePosted {
            group_id: 1,
            message_id: 42,
            message_count: 3,
        })),
    ])
    .await;
    let id = session
        .post_news(1, b"subject", b"line one\rline two")
        .await
        .unwrap();
    assert_eq!(id, 42);
    let sent =
        PostNewsMessage::decode(&server.payloads(MessageType::POST_NEWS_MESSAGE)[0]).unwrap();
    assert_eq!(sent.group_id, 1);
    assert_eq!(sent.subject, b"subject");
    assert_eq!(sent.body, b"line one\rline two");
    server.check();
}

#[tokio::test]
async fn post_converts_newlines_to_cr() {
    let (server, session) = session_with(vec![
        Step::Expect(MessageType::POST_NEWS_MESSAGE),
        Step::Send(Message::NewsMessagePosted(NewsMessagePosted {
            group_id: 1,
            message_id: 1,
            message_count: 1,
        })),
    ])
    .await;
    session.post_news(1, b"s", b"a\nb\r\nc").await.unwrap();
    let sent =
        PostNewsMessage::decode(&server.payloads(MessageType::POST_NEWS_MESSAGE)[0]).unwrap();
    assert_eq!(sent.body, b"a\rb\rc");
    server.check();
}

#[tokio::test]
async fn news_messages_and_one_message() {
    let msg = NewsMessage {
        group_id: 1,
        message_id: 5,
        poster: b"admin".to_vec(),
        subject: b"hi".to_vec(),
        body: b"body".to_vec(),
        padding: vec![0; 3],
        ..NewsMessage::default()
    };
    let (server, session) = session_with(vec![
        Step::Expect(MessageType::GET_NEWS_MESSAGES),
        Step::Reply(Message::NewsMessages(NewsMessages {
            group_id: 1,
            permissions: 7,
            ignored: [0; 4],
            messages: vec![msg.clone()],
        })),
        Step::Expect(MessageType::GET_NEWS_MESSAGE),
        Step::Reply(Message::NewsMessage(NewsMessageReply {
            message: msg.clone(),
        })),
    ])
    .await;
    let list = session.news_messages(1).await.unwrap();
    assert_eq!(list.messages[0].subject, b"hi");
    let one = session.news_message(1, 5).await.unwrap();
    assert_eq!(one.body, b"body");
    server.check();
}

#[tokio::test]
async fn post_without_613_times_out_cleanly() {
    let (server, mut session) =
        session_with(vec![Step::Expect(MessageType::POST_NEWS_MESSAGE)]).await;
    session.set_timeout(std::time::Duration::from_millis(200));
    let err = session.post_news(1, b"s", b"b").await.unwrap_err();
    assert!(matches!(err, ClientError::Timeout), "{err:?}");
    server.check();
}
