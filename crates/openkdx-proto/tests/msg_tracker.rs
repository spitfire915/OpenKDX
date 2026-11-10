//! Messages 700–721 (tracker.md).

mod common;

use common::{rejects_trailing, round_trip};
use openkdx_proto::msg::Payload;
use openkdx_proto::msg::common::Time1900;
use openkdx_proto::msg::tracker::*;

#[test]
fn no_payload_messages() {
    assert_eq!(round_trip(GetTrackerGroupList), Vec::<u8>::new());
    assert_eq!(round_trip(GetTrackerSettings), Vec::<u8>::new());
    assert_eq!(round_trip(SetTrackerSettingsDone), Vec::<u8>::new());
    assert_eq!(round_trip(GetTrackerStatistics), Vec::<u8>::new());
}

#[test]
fn group_list_entries_are_padded() {
    // 8 + 8 + 0 = 16 → no pad; 8 + 4 + 0 = 12 → none; 8 + 9 + 0 = 17 → 3
    let list = TrackerGroupList {
        groups: vec![
            GroupListEntry {
                servers: 0,
                value_02: [0; 4],
                name: b"Business".to_vec(),
                description: vec![],
                padding: vec![],
            },
            GroupListEntry {
                servers: 1,
                value_02: [0; 4],
                name: b"Education".to_vec(),
                description: vec![],
                padding: vec![0, 0, 0],
            },
        ],
    };
    let bytes = round_trip(list);
    assert_eq!(bytes.len(), 4 + 16 + 20);
}

#[test]
fn get_server_list_and_reply() {
    let bytes = round_trip(GetServerList {
        ignored_00: [0; 2],
        ignored_03: 0,
        group: b"General".to_vec(),
    });
    assert_eq!(bytes.len(), 11); // as captured
    assert_eq!(bytes[2], 7);

    // 0x22 + 10 + 0 + 0 = 44 → no pad
    let list = TrackerServerList {
        servers: vec![ServerListEntry {
            first_seen: Time1900(0x3a414c823b0),
            ignored_08: [0; 6],
            flags: 0,
            users: 0,
            ignored_12: [0; 6],
            address: 0x7f00_0001,
            port: 10700,
            name: b"New Server".to_vec(),
            description: vec![],
            extra: vec![],
            padding: vec![],
        }],
    };
    let bytes = round_trip(list);
    assert_eq!(bytes.len(), 48); // as captured
    assert_eq!(bytes[4 + 0x18], 6, "address_len is 6");
    assert_eq!(&bytes[4 + 0x1c..4 + 0x22], &[0x7f, 0, 0, 1, 0x29, 0xcc]);
    rejects_trailing::<TrackerServerList>(&bytes);
}

fn settings() -> TrackerSettings {
    let general = vec![0xaa; 0x50];
    let greeting = b"\0\0\0\0Welcome".to_vec();
    let mut sections: [Option<Vec<u8>>; 12] = Default::default();
    sections[0] = Some(general);
    sections[4] = Some(greeting);
    TrackerSettings {
        version: 1,
        ignored: [0; 2],
        sections,
    }
}

#[test]
fn settings_blob_has_a_section_table() {
    let s = settings();
    let bytes = round_trip(TrackerSettingsInfo {
        settings: s.clone(),
    });
    assert_eq!(&bytes[..8], b"%KDXTSTG");
    assert_eq!(&bytes[8..10], &[0, 1]);
    // section 0: offset 0x78, size 0x50; section 4: offset 0xc8, size 11
    assert_eq!(&bytes[0x0c..0x14], &[0, 0, 0, 0x78, 0, 0, 0, 0x50]);
    assert_eq!(&bytes[0x0c + 32..0x0c + 40], &[0, 0, 0, 0xc8, 0, 0, 0, 11]);
    assert_eq!(&bytes[0x0c + 8..0x0c + 16], &[0; 8], "absent section");
    assert_eq!(bytes[0x6c], 1);
    assert_eq!(bytes[0x6d], 0);
    assert_eq!(bytes[0x70], 1);
    assert_eq!(bytes.len(), 0x78 + 0x50 + 11);
    round_trip(SetTrackerSettings { settings: s });

    assert!(TrackerSettingsInfo::decode(&bytes[..0x77]).is_err());
    let mut bad = bytes.clone();
    bad[4] = b'X';
    assert!(TrackerSettingsInfo::decode(&bad).is_err());
}

#[test]
fn statistics() {
    let bytes = round_trip(TrackerStats {
        start_time: Time1900(1),
        connections_accepted: 2,
        datagrams_received: 3,
        ignored_18: [0; 8],
        hours: 4,
        connections: 5,
        servers: 6,
        ignored_2c: [0; 12],
    });
    assert_eq!(bytes.len(), 0x38);
}
