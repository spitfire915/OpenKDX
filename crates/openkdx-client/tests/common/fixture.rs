//! A logged-in session against the mock, for the per-area tests.
#![allow(dead_code)]

use std::time::Duration;

use openkdx_client::session::{ConnectOptions, Credentials, Session};
use openkdx_proto::msg::common::{Color, UserRecord};
use openkdx_proto::msg::session::{ServerInfo, UserList};
use openkdx_proto::{Message, MessageType};

use super::mock::{MockServer, Step, key, table};

pub fn user(id: u32, nick: &str) -> UserRecord {
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

pub fn server_info(user_id: u32) -> ServerInfo {
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

pub fn login_steps() -> Vec<Step> {
    vec![
        Step::Expect(MessageType::SET_IDENTITY),
        Step::Expect(MessageType::LICENSE_REPORT),
        Step::Send(Message::UserList(UserList {
            users: vec![user(1, "openkdx")],
        })),
        Step::Send(Message::ServerInfo(server_info(1))),
    ]
}

pub fn creds() -> Credentials {
    Credentials {
        login: b"admin".to_vec(),
        password: b"pw".to_vec(),
    }
}

pub fn opts() -> ConnectOptions {
    ConnectOptions {
        timeout: Duration::from_secs(5),
        table: Some(table()),
        ..ConnectOptions::default()
    }
}

/// A mock that runs login then `steps` on the first connection, and the
/// given scripts on later ones (transfer connections log in with the
/// 105 session key).
pub async fn session_with_extra(steps: Vec<Step>, extra: Vec<Vec<Step>>) -> (MockServer, Session) {
    let mut script = login_steps();
    script.extend(steps);
    let mut scripts = vec![script];
    scripts.extend(extra);
    let mut transfer_login = vec![0u8];
    transfer_login.extend_from_slice(&1u32.to_be_bytes());
    let server = MockServer::start(
        vec![
            (b"admin".to_vec(), key("pw")),
            (transfer_login, openkdx_proto::AccountKey([0xab; 32])),
        ],
        scripts,
    )
    .await;
    let session = Session::connect(server.addr, &creds(), opts())
        .await
        .unwrap();
    (server, session)
}

pub async fn session_with(steps: Vec<Step>) -> (MockServer, Session) {
    session_with_extra(steps, Vec::new()).await
}
