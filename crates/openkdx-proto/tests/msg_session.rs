//! Messages 100–118 (session.md).

mod common;

use common::{rejects_trailing, round_trip, user};
use openkdx_proto::msg::Payload;
use openkdx_proto::msg::common::Time1900;
use openkdx_proto::msg::session::*;

#[test]
fn set_identity_and_user_list() {
    // 0x1c + 8 + 7 + 4 = 47 bytes: the client sends no padding
    let bytes = round_trip(SetIdentity {
        user: user(0, b"labuser", &[]),
    });
    assert_eq!(bytes.len(), 47);
    rejects_trailing::<SetIdentity>(&bytes);

    // the server pads each record to a multiple of 4
    let bytes = round_trip(UserList {
        users: vec![user(1, b"labuser", &[0]), user(2, b"guest", &[0, 0, 0])],
    });
    assert_eq!(bytes.len(), 48 + 48);
    assert_eq!(UserList::decode(&[]).unwrap().users.len(), 0);
}

#[test]
fn user_changed_joined_departed() {
    round_trip(UserChanged {
        user: user(3, b"x", &[0, 0, 0]),
    });
    round_trip(UserJoined {
        user: user(3, b"x", &[0, 0, 0]),
    });
    let bytes = round_trip(UserDeparted {
        user_id: 3,
        kind: 7,
        ignored: [0; 3],
    });
    assert_eq!(bytes, [0, 0, 0, 3, 7, 0, 0, 0]);
    assert!(UserDeparted::decode(&bytes[..7]).is_err());
}

#[test]
fn server_info() {
    let info = ServerInfo {
        session_key: [7; 32],
        file_transfer_port: 0,
        user_id: 1,
        version: 0x0654,
        can_modify_settings: 1,
        ignored_2c: [0; 26],
        value_46: 2,
        ignored_4b: 0,
        name: b"New Server".to_vec(),
        description: vec![],
        address: vec![],
    };
    let bytes = round_trip(info);
    assert_eq!(bytes.len(), 86); // as captured
    assert_eq!(&bytes[..32], &[7; 32]);
    assert_eq!(&bytes[0x28..0x2a], &[0x06, 0x54]);
    assert_eq!(bytes[0x48], 10);
    rejects_trailing::<ServerInfo>(&bytes);
}

#[test]
fn icon_greeting_and_no_payload_messages() {
    round_trip(ServerIcon {
        icon: vec![1, 0x20, 0x20, 4],
    });
    round_trip(ServerIcon { icon: vec![] });
    let bytes = round_trip(ServerGreeting {
        ignored: [0; 4],
        text: b"(No greeting.)".to_vec(),
    });
    assert_eq!(&bytes[4..], b"(No greeting.)");

    assert_eq!(round_trip(Ping), Vec::<u8>::new());
    assert_eq!(round_trip(Pong), Vec::<u8>::new());
    assert_eq!(round_trip(RequestGreeting), Vec::<u8>::new());
    assert_eq!(round_trip(GetExtraServerInfo), Vec::<u8>::new());
    assert_eq!(round_trip(GetMyAddress), Vec::<u8>::new());
    assert!(Ping::decode(&[0]).is_err());
}

#[test]
fn mini_user_icon() {
    let icon = MiniUserIcon {
        status: 1,
        ignored: [0; 3],
    };
    assert_eq!(round_trip(SetMiniUserIcon { icon }), [1, 0, 0, 0]);
    assert_eq!(
        round_trip(MiniUserIconChanged { user_id: 3, icon }),
        [0, 0, 0, 3, 1, 0, 0, 0]
    );
}

#[test]
fn extra_server_info_and_your_address() {
    let bytes = round_trip(ExtraServerInfo {
        ignored_00: [0; 16],
        start_time: Time1900(0x3a36c4895ba),
        now: Time1900(0x3a36c4895bb),
        uptime: 1,
        os_kind: 0x1000,
        os_version: 1000,
        ignored_2d: [0; 3],
        os: b"Linux 6.1.0 #1 SMP".to_vec(),
    });
    assert_eq!(bytes.len(), 0x30 + 18);
    assert_eq!(bytes[0x2c], 18);

    let bytes = round_trip(YourAddress {
        address: b"127.0.0.1:58652".to_vec(),
    });
    assert_eq!(bytes[0], 15);
}

#[test]
fn skpk_and_license_report() {
    let bytes = round_trip(Skpk {
        key: [9; 32],
        dcc_port: 0x2968,
        connection_id: 2,
        client_version: 0x0654,
        ignored: [0; 6],
    });
    assert_eq!(bytes.len(), 48);
    assert_eq!(round_trip(LicenseReport { value: 1 }), [0, 0, 0, 1]);
}
