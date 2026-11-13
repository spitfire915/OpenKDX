#[path = "../../openkdx-client/tests/common/mock.rs"]
mod mock;

use std::io::Write;
use std::process::{Command, Stdio};

use mock::{MockServer, Step, key};
use openkdx_proto::msg::common::{Color, UserRecord};
use openkdx_proto::msg::session::{ServerInfo, UserList};
use openkdx_proto::{Message, MessageType};

/// A user record as the server sends it: padded to a multiple of 4.
fn user(id: u32, nick: &str) -> UserRecord {
    let mut user = UserRecord {
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
    };
    let mut w = openkdx_proto::Writer::new();
    user.encode(&mut w);
    user.padding = vec![0; openkdx_proto::wire::pad4_len(w.pos())];
    user
}

fn login_steps() -> Vec<Step> {
    vec![
        Step::Expect(MessageType::SET_IDENTITY),
        Step::Expect(MessageType::LICENSE_REPORT),
        Step::Send(Message::UserList(UserList {
            users: vec![user(1, "openkdx"), user(2, "bob")],
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

fn accounts() -> Vec<(Vec<u8>, openkdx_proto::AccountKey)> {
    vec![(b"admin".to_vec(), key("pw"))]
}

fn openkdx(server: &MockServer, args: &[&str], stdin: &str) -> (i32, String, String) {
    openkdx_as(server, "pw", args, stdin)
}

fn openkdx_as(
    server: &MockServer,
    password: &str,
    args: &[&str],
    stdin: &str,
) -> (i32, String, String) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_openkdx"));
    cmd.arg("--insecure-default-table")
        .arg("-s")
        .arg(server.addr.to_string())
        .arg("-l")
        .arg("admin")
        .arg("-p")
        .arg(password)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn login_prints_server_name() {
    let server = std::sync::Arc::new(MockServer::start(accounts(), vec![login_steps()]).await);
    let s2 = server.clone();
    let (code, stdout, stderr) = tokio::task::spawn_blocking(move || openkdx(&s2, &["login"], ""))
        .await
        .unwrap();
    server.check();
    assert_eq!(code, 0, "stdout {stdout}\nstderr {stderr}");
    assert!(stdout.contains("New Server"), "{stdout}");
    assert!(stdout.contains("1.620"), "{stdout}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wrong_password_exits_2() {
    let server = MockServer::start(accounts(), vec![vec![]]).await;
    let (code, _stdout, stderr) =
        tokio::task::spawn_blocking(move || openkdx_as(&server, "nope", &["login"], ""))
            .await
            .unwrap();
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("login failed"), "{stderr}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn run_executes_two_commands_on_one_session() {
    let server = MockServer::start(accounts(), vec![login_steps()]).await;
    let (code, stdout, stderr) = tokio::task::spawn_blocking(move || {
        openkdx(&server, &["run"], "users\n# comment\nusers\nquit\n")
    })
    .await
    .unwrap();
    assert_eq!(code, 0, "{stderr}");
    assert_eq!(stdout.matches("bob").count(), 2, "{stdout}");
    assert!(stdout.contains("> users"), "{stdout}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn json_users_is_one_object_per_line() {
    let server = MockServer::start(accounts(), vec![login_steps()]).await;
    let (code, stdout, _) =
        tokio::task::spawn_blocking(move || openkdx(&server, &["--json", "users"], ""))
            .await
            .unwrap();
    assert_eq!(code, 0);
    let lines: Vec<serde_json::Value> = stdout
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[1]["nick"], "bob");
    assert_eq!(lines[1]["user_id"], 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn chat_say_sends_207_and_run_can_pm_by_nick() {
    let mut steps = login_steps();
    steps.push(Step::Expect(MessageType::SEND_CHAT_TEXT));
    steps.push(Step::Expect(MessageType::SEND_PRIVATE_MESSAGE));
    let server = std::sync::Arc::new(MockServer::start(accounts(), vec![steps]).await);
    let s2 = server.clone();
    let (code, _stdout, stderr) = tokio::task::spawn_blocking(move || {
        openkdx(
            &s2,
            &["run"],
            "chat say 1 hello there\npm send bob hi bob\n",
        )
    })
    .await
    .unwrap();
    assert_eq!(code, 0, "{stderr}");
    server.check();
    let chat = server.payloads(MessageType::SEND_CHAT_TEXT);
    assert!(chat[0].ends_with(b"hello there"), "{chat:?}");
    let pm = server.payloads(MessageType::SEND_PRIVATE_MESSAGE);
    assert_eq!(&pm[0][..4], &[0, 0, 0, 2]);
    assert!(pm[0].ends_with(b"hi bob"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn news_post_reports_the_message_id() {
    use openkdx_proto::msg::news::NewsMessagePosted;
    let mut steps = login_steps();
    steps.push(Step::Expect(MessageType::POST_NEWS_MESSAGE));
    steps.push(Step::Send(Message::NewsMessagePosted(NewsMessagePosted {
        group_id: 1,
        message_id: 9,
        message_count: 1,
    })));
    let server = std::sync::Arc::new(MockServer::start(accounts(), vec![steps]).await);
    let s2 = server.clone();
    let (code, stdout, stderr) = tokio::task::spawn_blocking(move || {
        openkdx(
            &s2,
            &[
                "--json",
                "news",
                "post",
                "1",
                "--subject",
                "s",
                "--body",
                "b",
            ],
            "",
        )
    })
    .await
    .unwrap();
    assert_eq!(code, 0, "{stderr}");
    server.check();
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["message_id"], 9);
}

#[path = "../src/out.rs"]
#[allow(dead_code)]
mod out;

#[test]
fn time_string_formats_a_known_instant() {
    // `date -u -r 1794060399` = 2026-11-07 14:06:39.
    let t = openkdx_proto::msg::common::Time1900::from_unix_ms(1_794_060_399_000);
    assert_eq!(out::time_string(t), "2026-11-07 14:06:39");
    assert_eq!(out::version_string(0x0654), "1.620");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn files_ls_prints_a_listing_with_non_utf8_names() {
    use openkdx_proto::msg::files::{FileList, FileListEntry};
    let mut steps = login_steps();
    steps.push(Step::Expect(MessageType::GET_FILE_LIST));
    steps.push(Step::Reply(Message::FileList(FileList {
        entries: vec![
            FileListEntry {
                size: 3,
                attributes: FileListEntry::ATTR_FILE,
                name: b"caf\xa5.txt".to_vec(),
                kind: b"text/plain".to_vec(),
                padding: vec![0; 1],
                ..FileListEntry::default()
            },
            FileListEntry {
                size: 0x1000,
                attributes: FileListEntry::ATTR_FOLDER,
                access_item: 0x2b,
                name: b"Uploads".to_vec(),
                padding: vec![0; 2],
                ..FileListEntry::default()
            },
        ],
    })));
    let server = std::sync::Arc::new(MockServer::start(accounts(), vec![steps]).await);
    let s2 = server.clone();
    let (code, stdout, stderr) =
        tokio::task::spawn_blocking(move || openkdx(&s2, &["files", "ls", "-L"], ""))
            .await
            .unwrap();
    assert_eq!(code, 0, "{stderr}");
    server.check();
    assert!(stdout.contains("caf\u{fffd}.txt"), "{stdout}");
    assert!(stdout.contains("Uploads/"), "{stdout}");
    assert!(stdout.contains("[UL]"), "{stdout}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn files_get_and_put_use_a_transfer_connection() {
    use openkdx_proto::msg::files::{DownloadInfo, FileStrings, UploadFileComplete, UploadReply};
    let mut transfer_login = vec![0u8];
    transfer_login.extend_from_slice(&1u32.to_be_bytes());
    let mut accounts = accounts();
    accounts.push((transfer_login, openkdx_proto::AccountKey([0; 32])));
    let get_script = vec![
        Step::Expect(MessageType::DOWNLOAD_FILE),
        Step::Reply(Message::DownloadInfo(DownloadInfo {
            data_size: 3,
            strings: FileStrings {
                name: b"tiny.txt".to_vec(),
                ..FileStrings::default()
            },
            ..DownloadInfo::default()
        })),
        Step::DataReply {
            kind: MessageType::DOWNLOAD_FILE_DATA,
            reply: true,
            bytes: b"abc".to_vec(),
        },
        Step::DataReply {
            kind: MessageType::DOWNLOAD_FILE_DATA,
            reply: false,
            bytes: Vec::new(),
        },
    ];
    let put_script = vec![
        Step::Expect(MessageType::UPLOAD_FILE),
        Step::Reply(Message::UploadReply(UploadReply {
            data_offset: 0,
            rsrc_offset: u64::MAX,
            ignored: [0; 8],
        })),
        Step::ExpectData,
        Step::ExpectData,
        Step::Send(Message::UploadFileComplete(UploadFileComplete)),
    ];
    let server = std::sync::Arc::new(
        MockServer::start(accounts, vec![login_steps(), get_script, put_script]).await,
    );
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("tiny.txt");
    let up = dir.path().join("up.bin");
    std::fs::write(&up, b"hello upload").unwrap();
    let s2 = server.clone();
    let dest_s = dest.to_string_lossy().into_owned();
    let up_s = up.to_string_lossy().into_owned();
    let (code, stdout, stderr) = tokio::task::spawn_blocking(move || {
        openkdx(
            &s2,
            &["run"],
            &format!("files get tiny.txt --to {dest_s}\nfiles put {up_s} Uploads\n"),
        )
    })
    .await
    .unwrap();
    assert_eq!(code, 0, "{stdout}\n{stderr}");
    server.check();
    assert_eq!(std::fs::read(&dest).unwrap(), b"abc");
    assert_eq!(server.captured.lock().unwrap().data[0].1, b"hello upload");
    assert!(stdout.contains("3 bytes"), "{stdout}");
}
