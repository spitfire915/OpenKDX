mod common;

use std::time::Duration;

use common::mock::{MockServer, Step, key, table};
use openkdx_client::ClientError;
use openkdx_client::tracker::Tracker;
use openkdx_proto::msg::tracker::{
    GetServerList, GroupListEntry, ServerListEntry, TrackerGroupList, TrackerServerList,
};
use openkdx_proto::{Message, MessageType, Payload};

fn guest() -> Vec<(Vec<u8>, openkdx_proto::AccountKey)> {
    vec![(Vec::new(), key(""))]
}

#[tokio::test]
async fn tracker_rejects_server_role() {
    let server = MockServer::start_as(0x20, guest(), vec![vec![]]).await;
    let err = Tracker::connect(server.addr, &table(), Duration::from_secs(5))
        .await
        .unwrap_err();
    assert!(
        matches!(
            err,
            ClientError::Unexpected { .. } | ClientError::Protocol(_)
        ),
        "{err:?}"
    );
}

#[tokio::test]
async fn groups_and_servers_round_trip() {
    let server = MockServer::start_as(
        0x30,
        guest(),
        vec![vec![
            Step::Expect(MessageType::GET_TRACKER_GROUP_LIST),
            Step::Reply(Message::TrackerGroupList(TrackerGroupList {
                groups: vec![GroupListEntry {
                    servers: 1,
                    name: b"Default".to_vec(),
                    padding: vec![0; 1],
                    ..GroupListEntry::default()
                }],
            })),
            Step::Expect(MessageType::GET_TRACKER_SERVER_LIST),
            Step::Reply(Message::TrackerServerList(TrackerServerList {
                servers: vec![ServerListEntry {
                    users: 2,
                    address: u32::from_be_bytes([127, 0, 0, 1]),
                    port: 10700,
                    name: b"New Server".to_vec(),
                    padding: vec![0; 2],
                    ..ServerListEntry::default()
                }],
            })),
        ]],
    )
    .await;
    let tracker = Tracker::connect(server.addr, &table(), Duration::from_secs(5))
        .await
        .unwrap();
    let groups = tracker.groups().await.unwrap();
    assert_eq!(groups[0].name, b"Default");
    let servers = tracker.servers(b"Default").await.unwrap();
    assert_eq!(servers[0].socket_addr().to_string(), "127.0.0.1:10700");
    let sent =
        GetServerList::decode(&server.payloads(MessageType::GET_TRACKER_SERVER_LIST)[0]).unwrap();
    assert_eq!(sent.group, b"Default");
    tracker.close().await;
    server.check();
}
