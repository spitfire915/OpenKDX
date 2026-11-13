mod common;

use common::fixture::session_with;
use common::mock::Step;
use openkdx_client::path::{join, parse_path, path_string};
use openkdx_proto::msg::common::FilePath;
use openkdx_proto::msg::files::{FileList, FileListEntry, GetFileList};
use openkdx_proto::{Message, MessageType, Payload};

#[test]
fn parse_path_splits_on_slash() {
    let p = parse_path("a/b c/d.txt");
    assert_eq!(p.names.len(), 3);
    assert_eq!(p.names[1].name, b"b c");
    assert_eq!(path_string(&p), "a/b c/d.txt");
    assert!(parse_path("").is_root());
    assert!(parse_path("/").is_root());
    assert_eq!(parse_path("//x//").names.len(), 1);
    let j = join(&parse_path("a"), b"b");
    assert_eq!(path_string(&j), "a/b");
    assert_eq!(path_string(&join(&FilePath::root(), b"top")), "top");
}

#[tokio::test]
async fn list_root_sends_empty_path_and_decodes_entries() {
    let (server, session) = session_with(vec![
        Step::Expect(MessageType::GET_FILE_LIST),
        Step::Reply(Message::FileList(FileList {
            entries: vec![FileListEntry {
                size: 3,
                attributes: FileListEntry::ATTR_FILE,
                name: b"tiny.txt".to_vec(),
                padding: vec![0; 2],
                ..FileListEntry::default()
            }],
        })),
        Step::Expect(MessageType::GET_FILE_LIST),
        Step::Reply(Message::FileList(FileList {
            entries: Vec::new(),
        })),
    ])
    .await;
    let entries = session.list_files(&FilePath::root()).await.unwrap();
    assert_eq!(entries[0].name, b"tiny.txt");
    assert!(!entries[0].is_folder());
    let empty = session.list_files(&parse_path("Uploads")).await.unwrap();
    assert!(empty.is_empty());
    let sent = server.payloads(MessageType::GET_FILE_LIST);
    assert_eq!(sent[0], FilePath::root().to_bytes());
    assert_eq!(
        GetFileList::decode(&sent[1]).unwrap().path.names[0].name,
        b"Uploads"
    );
    server.check();
}

#[tokio::test]
async fn create_folder_returns_the_servers_answer() {
    use openkdx_proto::msg::Status;
    use openkdx_proto::msg::files::CreateFolder;
    let (server, session) = session_with(vec![
        Step::Expect(MessageType::CREATE_FOLDER),
        Step::Reply(Message::Status(Status {
            code: 0,
            request_type_or_value: 0x192,
            detail: None,
        })),
    ])
    .await;
    session
        .create_folder(&parse_path("Uploads"), b"New", b"", b"")
        .await
        .unwrap();
    let sent = CreateFolder::decode(&server.payloads(MessageType::CREATE_FOLDER)[0]).unwrap();
    assert_eq!(sent.name, b"New");
    assert_eq!(sent.path.names[0].name, b"Uploads");
    server.check();
}
