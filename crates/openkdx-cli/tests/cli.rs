//! The `openkdx` binary against `kdx_client::mock`: one scripted server per
//! test, the CLI run as a child process.

use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::Arc;

use kdx_client::mock::{MockConfig, MockServer, Step};
use kdx_proto::AccountKey;
use kdx_proto::msg::{ServerInfo, Status, UserRecord};
use kdx_proto::session::MessageType;

/// The session key the mock's 105 Server Info carries; transfer
/// connections log in with it.
const SESSION_KEY: [u8; 32] = [0xab; 32];

fn user(id: u32, nick: &str) -> UserRecord {
    UserRecord {
        user_id: id,
        client_port: 10600,
        nick: nick.as_bytes().to_vec(),
        ..UserRecord::default()
    }
}

fn login_steps() -> Vec<Step> {
    vec![
        Step::Expect(MessageType::SET_IDENTITY),
        Step::Send(
            MessageType::USER_LIST,
            UserRecord::encode_list(&[user(1, "openkdx"), user(2, "bob")]).unwrap(),
        ),
        Step::Send(
            MessageType::SERVER_INFO,
            ServerInfo {
                session_key: SESSION_KEY,
                user_id: 1,
                version: 0x0654,
                can_modify_settings: 1,
                value_46: 2,
                name: b"New Server".to_vec(),
                ..ServerInfo::default()
            }
            .encode()
            .unwrap(),
        ),
    ]
}

fn accounts() -> Vec<(Vec<u8>, AccountKey)> {
    vec![(b"admin".to_vec(), AccountKey::from_password(b"pw"))]
}

fn success(value: u32) -> Vec<u8> {
    Status::Success { value }.encode()
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
        .args(args);
    run(cmd, stdin)
}

fn run(mut cmd: Command, stdin: &str) -> (i32, String, String) {
    cmd.stdin(Stdio::piped())
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
    let server = Arc::new(MockServer::start(accounts(), vec![login_steps()]).await);
    let s2 = server.clone();
    let (code, stdout, stderr) = tokio::task::spawn_blocking(move || openkdx(&s2, &["login"], ""))
        .await
        .unwrap();
    server.finish().await;
    assert_eq!(code, 0, "stdout {stdout}\nstderr {stderr}");
    assert!(stdout.contains("New Server"), "{stdout}");
    assert!(stdout.contains("1.620"), "{stdout}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn wrong_password_exits_2() {
    // Server 1.620 answers a bad password with a proof of status 1.
    let config = MockConfig {
        reject: Some(kdx_proto::handshake::Status::Failed),
        ..MockConfig::default()
    };
    let server = MockServer::start_with(config, accounts(), vec![vec![]]).await;
    let (code, _stdout, stderr) =
        tokio::task::spawn_blocking(move || openkdx_as(&server, "nope", &["login"], ""))
            .await
            .unwrap();
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("login failed"), "{stderr}");
}

#[test]
fn login_without_a_table_explains_how_to_get_one() {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_openkdx"));
    cmd.env_remove("OPENKDX_KDX_TABLE")
        .env(
            "OPENKDX_TABLE_CACHE",
            std::env::temp_dir().join("openkdx-cli-no-table"),
        )
        .args(["-s", "127.0.0.1:1", "login"]);
    let (code, _stdout, stderr) = run(cmd, "");
    assert_eq!(code, 2, "{stderr}");
    assert!(stderr.contains("openkdx-table fetch"), "{stderr}");
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
    let server = Arc::new(MockServer::start(accounts(), vec![steps]).await);
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
    server.finish().await;
    let chat = server.payloads(MessageType::SEND_CHAT_TEXT);
    assert!(chat[0].ends_with(b"hello there"), "{chat:?}");
    let pm = server.payloads(MessageType::SEND_PRIVATE_MESSAGE);
    assert_eq!(&pm[0][..4], &[0, 0, 0, 2]);
    assert!(pm[0].ends_with(b"hi bob"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn news_post_reports_the_message_id() {
    use kdx_proto::msg::news::NewsMessageChange;
    let mut steps = login_steps();
    steps.push(Step::Expect(MessageType::POST_NEWS_MESSAGE));
    steps.push(Step::Send(
        MessageType::NEWS_MESSAGE_POSTED,
        NewsMessageChange {
            group_id: 1,
            message_id: 9,
            message_count: 1,
        }
        .encode()
        .to_vec(),
    ));
    steps.push(Step::Expect(MessageType::GET_MY_ADDRESS));
    steps.push(Step::Reply(MessageType::YOUR_ADDRESS, vec![127, 0, 0, 1]));
    let server = Arc::new(MockServer::start(accounts(), vec![steps]).await);
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
    server.finish().await;
    let v: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap();
    assert_eq!(v["message_id"], 9);
}

#[path = "../src/out.rs"]
#[allow(dead_code)]
mod out;

#[test]
fn time_string_formats_a_known_instant() {
    // `date -u -r 1794060399` = 2026-11-07 14:06:39.
    assert_eq!(
        out::time_string(1_794_060_399_000 + 2_208_988_800_000),
        "2026-11-07 14:06:39"
    );
    assert_eq!(out::version_string(0x0654), "1.620");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn files_ls_prints_a_listing_with_non_utf8_names() {
    use kdx_proto::msg::file::{ATTR_FILE, ATTR_FOLDER, FileListEntry};
    let mut steps = login_steps();
    steps.push(Step::Expect(MessageType::GET_FILE_LIST));
    steps.push(Step::Reply(
        MessageType::FILE_LIST,
        FileListEntry::encode_list(&[
            FileListEntry {
                size: 3,
                attributes: ATTR_FILE,
                name: b"caf\xa5.txt".to_vec(),
                kind: b"text/plain".to_vec(),
                ..FileListEntry::default()
            },
            FileListEntry {
                size: 0x1000,
                attributes: ATTR_FOLDER,
                access_item: 0x2b,
                name: b"Uploads".to_vec(),
                ..FileListEntry::default()
            },
        ])
        .unwrap(),
    ));
    let server = Arc::new(MockServer::start(accounts(), vec![steps]).await);
    let s2 = server.clone();
    let (code, stdout, stderr) =
        tokio::task::spawn_blocking(move || openkdx(&s2, &["files", "ls", "-L"], ""))
            .await
            .unwrap();
    assert_eq!(code, 0, "{stderr}");
    server.finish().await;
    assert!(stdout.contains("caf\u{fffd}.txt"), "{stdout}");
    assert!(stdout.contains("Uploads/"), "{stdout}");
    assert!(stdout.contains("[UL]"), "{stdout}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn files_get_and_put_use_a_transfer_connection() {
    use kdx_proto::msg::transfer::{DownloadInfo, FileStrings, UploadReply};
    let mut transfer_login = vec![0u8];
    transfer_login.extend_from_slice(&1u32.to_be_bytes());
    let mut accounts = accounts();
    accounts.push((transfer_login, AccountKey(SESSION_KEY)));
    let get_script = vec![
        Step::Expect(MessageType::DOWNLOAD_FILE),
        Step::Reply(
            MessageType::DOWNLOAD_FILE_REPLY,
            DownloadInfo {
                data_size: 3,
                strings: FileStrings {
                    name: b"tiny.txt".to_vec(),
                    ..FileStrings::default()
                },
                ..DownloadInfo::default()
            }
            .encode()
            .unwrap(),
        ),
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
        Step::Reply(
            MessageType::UPLOAD_FILE_REPLY,
            UploadReply {
                data_offset: 0,
                rsrc_offset: u64::MAX,
                ..UploadReply::default()
            }
            .encode()
            .to_vec(),
        ),
        Step::ExpectData,
        Step::ExpectData,
        Step::Send(MessageType::UPLOAD_FILE_COMPLETE, Vec::new()),
    ];
    let server =
        Arc::new(MockServer::start(accounts, vec![login_steps(), get_script, put_script]).await);
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
    server.finish().await;
    assert_eq!(std::fs::read(&dest).unwrap(), b"abc");
    assert_eq!(server.captured.lock().unwrap().data[0].1, b"hello upload");
    assert!(stdout.contains("3 bytes"), "{stdout}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn accounts_create_sends_507_with_the_password_key() {
    use kdx_proto::msg::admin::Account;
    use kdx_proto::msg::privileges::Privilege;
    let mut steps = login_steps();
    steps.push(Step::Expect(MessageType::CREATE_ACCOUNT));
    steps.push(Step::Reply(MessageType::STATUS, success(0x1fb)));
    let server = Arc::new(MockServer::start(accounts(), vec![steps]).await);
    let s2 = server.clone();
    let (code, _stdout, stderr) = tokio::task::spawn_blocking(move || {
        openkdx(
            &s2,
            &[
                "accounts",
                "create",
                "tester",
                "--new-password",
                "t",
                "--name",
                "Tester",
                "--class",
                "1",
                "--privileges",
                "can-login,download",
            ],
            "",
        )
    })
    .await
    .unwrap();
    assert_eq!(code, 0, "{stderr}");
    server.finish().await;
    let sent = Account::decode(&server.payloads(MessageType::CREATE_ACCOUNT)[0]).unwrap();
    assert_eq!(sent.login, b"tester");
    assert_eq!(sent.class_index, 1);
    assert_eq!(sent.key, AccountKey::from_password(b"t").0);
    assert!(sent.privileges.contains(Privilege::Download));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn server_broadcast_and_disconnect_send_154_and_150() {
    use kdx_proto::msg::admin::DisconnectUser;
    let mut steps = login_steps();
    steps.push(Step::Expect(MessageType::SEND_BROADCAST_MESSAGE));
    steps.push(Step::Expect(MessageType::GET_MY_ADDRESS));
    steps.push(Step::Reply(MessageType::YOUR_ADDRESS, vec![127, 0, 0, 1]));
    steps.push(Step::Expect(MessageType::DISCONNECT_USER));
    steps.push(Step::Reply(MessageType::STATUS, success(150)));
    let server = Arc::new(MockServer::start(accounts(), vec![steps]).await);
    let s2 = server.clone();
    let (code, _stdout, stderr) = tokio::task::spawn_blocking(move || {
        openkdx(
            &s2,
            &["run"],
            "server broadcast hello all\nserver disconnect bob --ban 10 --message bye\n",
        )
    })
    .await
    .unwrap();
    assert_eq!(code, 0, "{stderr}");
    server.finish().await;
    assert!(server.payloads(MessageType::SEND_BROADCAST_MESSAGE)[0].ends_with(b"hello all"));
    let d = DisconnectUser::decode(&server.payloads(MessageType::DISCONNECT_USER)[0]).unwrap();
    assert_eq!((d.user_id, d.ban_seconds), (2, 10));
    assert_eq!(d.message, b"bye");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tracker_servers_prints_addresses() {
    use kdx_proto::msg::tracker::ServerListEntry;
    let tracker = Arc::new(
        MockServer::start_as(
            0x30,
            vec![(Vec::new(), AccountKey::from_password(b""))],
            vec![vec![
                Step::Expect(MessageType::GET_TRACKER_SERVER_LIST),
                Step::Reply(
                    MessageType::TRACKER_SERVER_LIST,
                    ServerListEntry::encode_list(&[ServerListEntry {
                        first_seen: 0,
                        ignored_08: [0; 6],
                        flags: 0,
                        users: 2,
                        ignored_12: [0; 6],
                        address: std::net::Ipv4Addr::new(127, 0, 0, 1),
                        port: 10700,
                        name: b"New Server".to_vec(),
                        description: Vec::new(),
                        extra: Vec::new(),
                        padding: Vec::new(),
                    }])
                    .unwrap(),
                ),
            ]],
        )
        .await,
    );
    let t2 = tracker.clone();
    let (code, stdout, stderr) = tokio::task::spawn_blocking(move || {
        let addr = t2.addr.to_string();
        openkdx(
            &t2,
            &["tracker", "--tracker", &addr, "servers", "Default"],
            "",
        )
    })
    .await
    .unwrap();
    assert_eq!(code, 0, "{stderr}");
    tracker.finish().await;
    assert!(stdout.contains("127.0.0.1:10700"), "{stdout}");
    assert!(stdout.contains("New Server"), "{stdout}");
}
