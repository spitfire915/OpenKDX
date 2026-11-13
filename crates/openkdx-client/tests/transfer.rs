mod common;

use common::fixture::session_with_extra;
use common::mock::Step;
use openkdx_client::ClientError;
use openkdx_client::path::parse_path;
use openkdx_client::transfer::Transfer;
use openkdx_proto::msg::files::{
    DownloadFile, DownloadInfo, FileStrings, UploadFile, UploadFileComplete, UploadReply,
};
use openkdx_proto::{Message, MessageType, Payload};

fn info(size: u64) -> DownloadInfo {
    DownloadInfo {
        data_size: size,
        strings: FileStrings {
            name: b"big.bin".to_vec(),
            second: b"text/plain".to_vec(),
            ..FileStrings::default()
        },
        ..DownloadInfo::default()
    }
}

fn download_script(size: u64, fork: Vec<u8>) -> Vec<Step> {
    vec![
        Step::Expect(MessageType::DOWNLOAD_FILE),
        Step::Reply(Message::DownloadInfo(info(size))),
        Step::DataReply {
            kind: MessageType::DOWNLOAD_FILE_DATA,
            reply: true,
            bytes: fork,
        },
        Step::DataReply {
            kind: MessageType::DOWNLOAD_FILE_DATA,
            reply: false,
            bytes: Vec::new(),
        },
    ]
}

fn body(n: usize) -> Vec<u8> {
    (0..n).map(|i| (i % 251) as u8).collect()
}

#[tokio::test]
async fn download_writes_fork_bytes() {
    let data = body(5000);
    let (server, session) =
        session_with_extra(vec![], vec![download_script(5000, data.clone())]).await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("big.bin");
    let mut t = Transfer::open(&session).await.unwrap();
    let mut seen = Vec::new();
    let got = t
        .download(&parse_path("big.bin"), &dest, false, |p| seen.push(p.done))
        .await
        .unwrap();
    assert_eq!(got.data_size, 5000);
    assert_eq!(std::fs::read(&dest).unwrap(), data);
    assert_eq!(*seen.last().unwrap(), 5000);
    let req = DownloadFile::decode(&server.payloads(MessageType::DOWNLOAD_FILE)[0]).unwrap();
    assert_eq!(req.data_offset, 0);
    assert_eq!(req.rsrc_offset, u64::MAX);
    assert_eq!(req.path.names[0].name, b"big.bin");
    server.check();
}

#[tokio::test]
async fn download_resume_from_rounded_offset() {
    // The local file has 5000 bytes; the server rounds the offset down to
    // 0x1000 and sends the rest from there.
    let data = body(9000);
    let (server, session) =
        session_with_extra(vec![], vec![download_script(9000, data[0x1000..].to_vec())]).await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("big.bin");
    let mut partial = data[..0x1000].to_vec();
    partial.extend_from_slice(b"garbage after the page boundary that the server resends");
    std::fs::write(&dest, &partial).unwrap();
    let mut t = Transfer::open(&session).await.unwrap();
    t.download(&parse_path("big.bin"), &dest, true, |_| {})
        .await
        .unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), data);
    let req = DownloadFile::decode(&server.payloads(MessageType::DOWNLOAD_FILE)[0]).unwrap();
    assert_eq!(req.data_offset, partial.len() as u64);
    server.check();
}

#[tokio::test]
async fn download_resume_at_end_leaves_file_intact() {
    let (server, session) = session_with_extra(vec![], vec![download_script(3, Vec::new())]).await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("tiny.txt");
    std::fs::write(&dest, b"abc").unwrap();
    let mut t = Transfer::open(&session).await.unwrap();
    t.download(&parse_path("tiny.txt"), &dest, true, |_| {})
        .await
        .unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), b"abc");
    server.check();
}

#[tokio::test]
async fn download_without_resume_overwrites() {
    let (server, session) =
        session_with_extra(vec![], vec![download_script(3, b"xyz".to_vec())]).await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("tiny.txt");
    std::fs::write(&dest, b"a much longer old file").unwrap();
    let mut t = Transfer::open(&session).await.unwrap();
    t.download(&parse_path("tiny.txt"), &dest, false, |_| {})
        .await
        .unwrap();
    assert_eq!(std::fs::read(&dest).unwrap(), b"xyz");
    server.check();
}

#[tokio::test]
async fn download_into_a_directory_uses_the_server_name() {
    let (server, session) =
        session_with_extra(vec![], vec![download_script(3, b"xyz".to_vec())]).await;
    let dir = tempfile::tempdir().unwrap();
    let mut t = Transfer::open(&session).await.unwrap();
    let written = t
        .download_to_dir(&parse_path("Uploads/big.bin"), dir.path(), false, |_| {})
        .await
        .unwrap();
    assert_eq!(written, dir.path().join("big.bin"));
    assert_eq!(std::fs::read(&written).unwrap(), b"xyz");
    server.check();
}

fn upload_script(offset: u64) -> Vec<Step> {
    vec![
        Step::Expect(MessageType::UPLOAD_FILE),
        Step::Reply(Message::UploadReply(UploadReply {
            data_offset: offset,
            rsrc_offset: u64::MAX,
            ignored: [0; 8],
        })),
        Step::ExpectData,
        Step::ExpectData,
        Step::Send(Message::UploadFileComplete(UploadFileComplete)),
    ]
}

#[tokio::test]
async fn upload_sends_417_for_each_fork_and_waits_418() {
    let data = body(70_000);
    let (server, session) = session_with_extra(vec![], vec![upload_script(0)]).await;
    let dir = tempfile::tempdir().unwrap();
    let local = dir.path().join("big.bin");
    std::fs::write(&local, &data).unwrap();
    let mut t = Transfer::open(&session).await.unwrap();
    let mut last = 0;
    t.upload(
        &local,
        &parse_path("Uploads"),
        b"big.bin",
        false,
        |p| last = p.done,
        None,
    )
    .await
    .unwrap();
    assert_eq!(last, 70_000);
    let req = UploadFile::decode(&server.payloads(MessageType::UPLOAD_FILE)[0]).unwrap();
    assert_eq!(req.data_size, 70_000);
    assert_eq!(req.rsrc_size, 0);
    assert_eq!(req.strings.name, b"big.bin");
    assert_eq!(req.strings.second, b"exttype/bin");
    assert_eq!(req.folder.names[0].name, b"Uploads");
    let captured = server.captured.lock().unwrap();
    assert_eq!(captured.data.len(), 2);
    assert_eq!(captured.data[0].0, 70_000);
    assert_eq!(captured.data[0].1, data);
    assert_eq!(captured.data[1].0, 0);
    // The data fork header replies to 415's txid.
    let txid_415 = captured
        .messages
        .iter()
        .find(|m| m.0 == MessageType::UPLOAD_FILE)
        .unwrap()
        .1;
    assert_eq!(captured.data_headers[0], (txid_415, true));
    assert!(!captured.data_headers[1].1);
    drop(captured);
    server.check();
}

#[tokio::test]
async fn upload_resumes_from_416_offset() {
    let data = body(70_000);
    let (server, session) = session_with_extra(vec![], vec![upload_script(0x4000)]).await;
    let dir = tempfile::tempdir().unwrap();
    let local = dir.path().join("big.bin");
    std::fs::write(&local, &data).unwrap();
    let mut t = Transfer::open(&session).await.unwrap();
    t.upload(
        &local,
        &parse_path("Uploads"),
        b"big.bin",
        true,
        |_| {},
        None,
    )
    .await
    .unwrap();
    let captured = server.captured.lock().unwrap();
    assert_eq!(captured.data[0].0, 70_000 - 0x4000);
    assert_eq!(captured.data[0].1, data[0x4000..]);
    drop(captured);
    server.check();
}

#[tokio::test]
async fn upload_stop_after_closes_early() {
    let data = body(70_000);
    let (server, session) = session_with_extra(
        vec![],
        vec![vec![
            Step::Expect(MessageType::UPLOAD_FILE),
            Step::Reply(Message::UploadReply(UploadReply {
                data_offset: 0,
                rsrc_offset: u64::MAX,
                ignored: [0; 8],
            })),
        ]],
    )
    .await;
    let dir = tempfile::tempdir().unwrap();
    let local = dir.path().join("big.bin");
    std::fs::write(&local, &data).unwrap();
    let mut t = Transfer::open(&session).await.unwrap();
    let mut last = 0;
    let err = t
        .upload(
            &local,
            &parse_path("Uploads"),
            b"big.bin",
            false,
            |p| last = p.done,
            Some(20_000),
        )
        .await
        .unwrap_err();
    assert!(matches!(err, ClientError::Transfer(_)), "{err:?}");
    assert!((20_000..70_000).contains(&last), "{last}");
    server.check();
}

#[tokio::test]
async fn upload_name_exists_is_a_server_error() {
    use openkdx_proto::msg::Status;
    let (server, session) = session_with_extra(
        vec![],
        vec![vec![
            Step::Expect(MessageType::UPLOAD_FILE),
            Step::Reply(Message::Status(Status {
                code: 0x30073,
                request_type_or_value: 415,
                detail: None,
            })),
        ]],
    )
    .await;
    let dir = tempfile::tempdir().unwrap();
    let local = dir.path().join("x.txt");
    std::fs::write(&local, b"x").unwrap();
    let mut t = Transfer::open(&session).await.unwrap();
    let err = t
        .upload(&local, &parse_path(""), b"x.txt", false, |_| {}, None)
        .await
        .unwrap_err();
    assert!(
        matches!(err, ClientError::Server { code: 0x30073, .. }),
        "{err:?}"
    );
    server.check();
}
