//! Messages 140–143 (monitoring.md) and 150–191 (administration.md).

mod common;

use common::{rejects_trailing, round_trip};
use openkdx_proto::MessageType;
use openkdx_proto::msg::Payload;
use openkdx_proto::msg::admin::*;
use openkdx_proto::msg::common::{FilePath, PathName, Time1900};

#[test]
fn no_payload_messages() {
    assert_eq!(round_trip(OpenConnectionMonitor), Vec::<u8>::new());
    assert_eq!(round_trip(CloseConnectionMonitor), Vec::<u8>::new());
    assert_eq!(round_trip(GetConnectionMonitorInfo), Vec::<u8>::new());
    assert_eq!(round_trip(GetServerSettings), Vec::<u8>::new());
    assert_eq!(round_trip(SetServerSettingsDone), Vec::<u8>::new());
    assert_eq!(round_trip(UpgradeServerSoftware), Vec::<u8>::new());
    assert_eq!(round_trip(ServerHistoryJoin), Vec::<u8>::new());
    assert_eq!(round_trip(ServerHistoryLeave), Vec::<u8>::new());
}

fn monitored(transfer: bool, pad: &[u8]) -> MonitoredConnection {
    MonitoredConnection {
        user_id: 1,
        transfer_id: u32::from(transfer),
        login_time: Time1900(1),
        time_10: Time1900(0),
        time_18: Time1900(2),
        bytes_done: 3,
        bytes_total: 70000,
        rate: 0x51a1,
        seconds_left: 3,
        value_38: 0x5000,
        state: 0x19c,
        ignored_40: [0; 2],
        class_index: 0,
        ignored_43: [0; 29],
        ignored_64: [0; 2],
        nick: b"probe".to_vec(),
        login: b"admin".to_vec(),
        account: b"Administrator".to_vec(),
        address: b"127.0.0.1:50984".to_vec(),
        path: if transfer {
            FilePath {
                names: vec![PathName::new(b"probe-dl.bin")],
            }
        } else {
            FilePath::root()
        },
        padding: pad.to_vec(),
    }
}

#[test]
fn connection_monitor_info_records() {
    // 0x68 + 5 + 5 + 13 + 15 = 142 → 2 bytes; transfer adds 2 + 3 + 12 = 17 → 159 → 1 byte
    let bytes = round_trip(ConnectionMonitorInfo {
        records: vec![monitored(false, &[0, 0]), monitored(true, &[0])],
    });
    assert_eq!(bytes.len(), 144 + 160);
    assert_eq!(&bytes[0x66..0x68], &[0, 0], "root path has length 0");
    assert_eq!(&bytes[144 + 0x66..144 + 0x68], &[0, 17]);
    assert_eq!(
        round_trip(ConnectionMonitorInfo { records: vec![] }),
        Vec::<u8>::new()
    );
}

#[test]
fn disconnects_and_user_info() {
    let bytes = round_trip(DisconnectUser {
        user_id: 2,
        ignored_04: [0; 4],
        ban_seconds: 600,
        ignored_0c: [0; 5],
        options: 0,
        ignored_12: [0; 2],
        ignored_15: [0; 3],
        message: b"bye".to_vec(),
    });
    assert_eq!(bytes.len(), 0x18 + 3);
    assert_eq!(bytes[0x14], 3);
    rejects_trailing::<DisconnectUser>(&bytes);
    let bytes = round_trip(DisconnectMessage {
        user_id: 2,
        ignored: [0; 8],
        flags: 2,
        value_0e: 0,
        value_10: 0,
        message: b"bye".to_vec(),
        name: b"labuser".to_vec(),
    });
    assert_eq!(bytes.len(), 0x14 + 3 + 7);
    assert_eq!(round_trip(GetUserInfo { user_id: 3 }), [0, 0, 0, 3]);
    // 0x44 + 15 + 5 + 13 + 5 + 13 = 119 → 1 byte, uninitialized
    let bytes = round_trip(UserInfo {
        user_id: 1,
        ignored_04: [0; 4],
        login_time: Time1900(1),
        time_10: Time1900(2),
        time_18: Time1900(3),
        ignored_20: [0; 27],
        ignored_3c: 0,
        ignored_3e: 0,
        ignored_42: [0; 2],
        address: b"127.0.0.1:50984".to_vec(),
        nick: b"probe".to_vec(),
        name_3f: b"Administrator".to_vec(),
        login: b"admin".to_vec(),
        name_41: b"Administrator".to_vec(),
        padding: vec![0x5a],
    });
    assert_eq!(bytes.len(), 120);
    assert_eq!(bytes[119], 0x5a);
}

#[test]
fn broadcasts() {
    let bytes = round_trip(SendBroadcast {
        value_0: 0,
        value_2: 7,
        text: b"hello all".to_vec(),
    });
    assert_eq!(bytes.len(), 4 + 9);
    let bytes = round_trip(RcvBroadcast {
        user_id: 0,
        ignored: [0; 10],
        value_0e: 7,
        text: b"hello all".to_vec(),
    });
    assert_eq!(bytes.len(), 0x10 + 9);
    assert!(RcvBroadcast::decode(&[0; 15]).is_err());
}

fn settings() -> ServerSettings {
    ServerSettings {
        version: 1,
        ignored: [0; 2],
        table: [
            (0x70, 0x68),
            (0xd8, 4),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
            (0, 0),
        ],
        value_64: [1, 1, 1, 1, 0, 1, 0, 2, 0, 0, 0, 0],
        body: {
            let mut b = vec![0u8; 0x68];
            b[0x23] = 0x02;
            b[0x38..0x3c].copy_from_slice(&0x5000u32.to_be_bytes());
            b[0x52] = 10;
            b[0x5c..0x5c + 10].copy_from_slice(b"New Server");
            b.extend_from_slice(&[9, 9, 9, 9]);
            b
        },
    }
}

#[test]
fn server_settings_blob() {
    let s = settings();
    let bytes = round_trip(ServerSettingsInfo {
        settings: s.clone(),
    });
    assert_eq!(&bytes[..8], b"%KDXSSTG");
    assert_eq!(bytes.len(), 0x70 + 0x68 + 4);
    assert_eq!(s.section(0).unwrap().len(), 0x68);
    assert_eq!(s.section(1).unwrap(), &[9, 9, 9, 9]);
    assert!(s.section(2).is_none());
    assert_eq!(s.main().unwrap().server_name, b"New Server");
    assert_eq!(s.main().unwrap().total_outgoing_speed_limit, 0x5000);
    assert!(s.main().unwrap().greeting_enabled());
    round_trip(SetServerSettings { settings: s });
    assert!(ServerSettingsInfo::decode(&bytes[..0x6f]).is_err());
    let mut bad = bytes.clone();
    bad[5] = b'X';
    assert!(ServerSettingsInfo::decode(&bad).is_err());
}

#[test]
fn shutdown() {
    let bytes = round_trip(ShutdownServer {
        ignored_00: 0,
        mode: 1,
        ignored_02: [0; 2],
        message: b"maintenance".to_vec(),
    });
    assert_eq!(bytes.len(), 4 + 11);
    assert!(ShutdownServer::decode(&[0; 3]).is_err());
}

fn record(kind: u32, pad: &[u8]) -> HistoryRecord {
    HistoryRecord {
        time: Time1900(0x3a36c4895ba),
        kind: HistoryKind(kind),
        user_id: 1,
        ignored_10: [0; 50],
        ignored_44: [0; 2],
        ignored_4a: [0; 2],
        address: b"127.0.0.1:1".to_vec(),
        nick: b"labuser".to_vec(),
        login: b"admin".to_vec(),
        detail: b"Public Chat".to_vec(),
        path: FilePath::root(),
        padding: pad.to_vec(),
    }
}

#[test]
fn history() {
    // 0x4c + 11 + 7 + 5 + 11 = 110 → 2 bytes
    let bytes = round_trip(HistoryList {
        ignored: 0,
        records: vec![record(190, &[0, 0]), record(184, &[0, 0])],
    });
    assert_eq!(bytes.len(), 8 + 112 + 112);
    assert_eq!(&bytes[8 + 8..8 + 12], &[0, 0, 0, 190]);
    let bytes = round_trip(HistoryAdded {
        record: record(188, &[]),
    });
    assert_eq!(bytes.len(), 110);
    assert_eq!(HistoryKind(190).name(), Some("Server Started"));
    assert_eq!(HistoryKind(191).name(), Some("Login Failed"));
    assert_eq!(HistoryKind(5).name(), None);
    assert_eq!(MessageType(184).name(), Some("Login"));
    assert_eq!(MessageType(186).name(), Some("Abort File Download"));
}
