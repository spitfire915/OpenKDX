//! Messages 600–614 (news.md).

mod common;

use common::{rejects_trailing, round_trip};
use openkdx_proto::msg::Payload;
use openkdx_proto::msg::common::Time1900;
use openkdx_proto::msg::news::*;

fn group() -> NewsgroupInfo {
    NewsgroupInfo {
        group_id: 1,
        ignored_04: [0; 12],
        time: Time1900(5),
        read_classes: 0xffff_ffff,
        post_classes: 0xffff_ffff,
        ignored_20: [1, 0, 0, 0],
        ignored_24: [0; 12],
        message_count: 1,
        size_limit_kib: 0x32,
        ignored_34: [0; 8],
        ignored_3e: [0; 2],
        name: b"General News".to_vec(),
        description: vec![],
    }
}

fn message(subject: &[u8], body: &[u8], pad: &[u8]) -> NewsMessage {
    NewsMessage {
        time: Time1900(0x3a36c4895ba),
        group_id: 1,
        message_id: 2,
        ignored_10: [0; 8],
        ignored_1a: [0; 2],
        ignored_1f: 0,
        poster: b"labuser".to_vec(),
        login: b"admin".to_vec(),
        subject: subject.to_vec(),
        body: body.to_vec(),
        padding: pad.to_vec(),
    }
}

#[test]
fn newsgroup_info_in_create_get_and_set() {
    let bytes = round_trip(CreateNewsgroup { group: group() });
    assert_eq!(bytes.len(), 0x40 + 12); // 76 bytes as seen for a 12-byte name
    assert_eq!(bytes[0x3c], 12);
    rejects_trailing::<CreateNewsgroup>(&bytes);
    round_trip(NewsgroupInfoReply { group: group() });
    round_trip(SetNewsgroup {
        group: NewsgroupInfo {
            description: b"edited".to_vec(),
            ..group()
        },
    });
}

#[test]
fn group_ids_and_no_payload() {
    assert_eq!(round_trip(DeleteNewsgroup { group_id: 1 }), [0, 0, 0, 1]);
    assert_eq!(round_trip(GetNewsgroup { group_id: 1 }), [0, 0, 0, 1]);
    assert_eq!(round_trip(GetNewsMessages { group_id: 1 }), [0, 0, 0, 1]);
    assert_eq!(round_trip(GetNewsgroupList), Vec::<u8>::new());
    assert_eq!(
        round_trip(GetNewsMessage {
            group_id: 1,
            message_id: 2
        }),
        [0, 0, 0, 1, 0, 0, 0, 2]
    );
    assert_eq!(
        round_trip(DeleteNewsMessage {
            group_id: 1,
            message_id: 2
        }),
        [0, 0, 0, 1, 0, 0, 0, 2]
    );
}

#[test]
fn newsgroup_list_entries_are_padded() {
    // 0x18 + 12 + 0 = 36 → no padding; 0x18 + 5 + 0 = 29 → 3 bytes
    let list = NewsgroupList {
        groups: vec![
            NewsgroupEntry {
                time: Time1900(1),
                group_id: 1,
                ignored_0c: [0; 4],
                message_count: 1,
                ignored_12: [0; 4],
                name: b"General News".to_vec(),
                description: vec![],
                padding: vec![],
            },
            NewsgroupEntry {
                time: Time1900(2),
                group_id: 2,
                ignored_0c: [0; 4],
                message_count: 0,
                ignored_12: [0; 4],
                name: b"Other".to_vec(),
                description: vec![],
                padding: vec![0xaa, 0xbb, 0xcc],
            },
        ],
    };
    let bytes = round_trip(list);
    assert_eq!(bytes.len(), 4 + 36 + 32); // 40 bytes with one group, as captured
    assert_eq!(&bytes[bytes.len() - 3..], &[0xaa, 0xbb, 0xcc]);
}

#[test]
fn news_messages_list_and_single() {
    // 0x20 + 7 + 5 + 7 + 10 = 61 → 3 bytes of padding
    let m = message(b"Lab post", b"Posted now", &[0, 0]);
    let m = NewsMessage {
        subject: b"Lab pos".to_vec(),
        padding: vec![0, 0, 0],
        ..m
    };
    let bytes = round_trip(NewsMessages {
        group_id: 1,
        permissions: 7,
        ignored: [0; 4],
        messages: vec![m.clone(), m.clone()],
    });
    assert_eq!(bytes.len(), 0x10 + 64 + 64);
    assert_eq!(&bytes[0x28..0x2a], &[0, 10], "body_len is a u16 at 0x18 of the record");

    // a user who may post but not read gets only the 16-byte header
    let mut header = bytes[..16].to_vec();
    header[12..].copy_from_slice(&[0, 0, 0, 0]);
    let only_header = NewsMessages::decode(&header).unwrap();
    assert!(only_header.messages.is_empty());
    assert_eq!(only_header.to_bytes(), header);

    let bytes = round_trip(NewsMessageReply { message: m });
    assert_eq!(bytes.len(), 64);
    assert!(
        NewsMessageReply::decode(&bytes[..61])
            .unwrap()
            .message
            .padding
            .is_empty(),
        "a lone 610 may come unpadded"
    );
}

#[test]
fn post_and_change_notices() {
    let bytes = round_trip(PostNewsMessage {
        group_id: 1,
        ignored_04: [0; 8],
        ignored_0f: 0,
        subject: b"Lab post".to_vec(),
        body: b"Posted from the lab scenario.".to_vec(),
        rest: vec![],
    });
    assert_eq!(bytes.len(), 0x10 + 8 + 29); // 53, as captured
    assert_eq!(&bytes[0x0c..0x0e], &[0, 29]);
    assert_eq!(bytes[0x0e], 8);

    assert_eq!(
        round_trip(NewsMessagePosted {
            group_id: 1,
            message_id: 2,
            message_count: 2
        }),
        [0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 2]
    );
    round_trip(NewsMessageDeleted {
        group_id: 1,
        message_id: 2,
        message_count: 1,
    });
}
