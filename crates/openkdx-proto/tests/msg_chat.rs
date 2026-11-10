//! Messages 200–223 (chat.md) and 300–301 (private-messages.md).

mod common;

use common::{rejects_trailing, round_trip};
use openkdx_proto::msg::Payload;
use openkdx_proto::msg::chat::*;
use openkdx_proto::msg::common::{ChatRecord, Time1900};

fn chat_record() -> ChatRecord {
    ChatRecord {
        chat_id: 0,
        key: 0,
        ignored_08: [0; 12],
        join_classes: 0xffff_ffff,
        send_classes: 0xffff_ffff,
        topic_classes: 0xffff_ffff,
        ignored_20: [0; 8],
        flags: 0,
        ignored_2e: [0; 2],
        name: b"lab-room".to_vec(),
        description: b"lab".to_vec(),
        irc_server: vec![],
        irc_channel: vec![],
    }
}

fn window_info() -> ChatWindowInfo {
    ChatWindowInfo {
        time: Time1900(0x3a36c4895ba),
        chat_id: 1,
        key: 0,
        ignored_10: [0; 4],
        flags: 0,
        ignored_18: [0; 6],
        ignored_23: 0,
        members: vec![1],
        name: b"Public Chat".to_vec(),
        topic: b"lab topic".to_vec(),
        topic_setter: b"labuser".to_vec(),
        irc_server: b"irc.x:6667".to_vec(),
        irc_channel: b"#k".to_vec(),
    }
}

#[test]
fn chat_ids_and_no_payload_messages() {
    assert_eq!(round_trip(CreatePrivateChat), Vec::<u8>::new());
    assert_eq!(round_trip(MustJoinChat), Vec::<u8>::new());
    assert_eq!(round_trip(GetChatList), Vec::<u8>::new());
    assert_eq!(round_trip(DeleteChat { chat_id: 5 }), [0, 0, 0, 5]);
    assert_eq!(round_trip(LeaveChat { chat_id: 5 }), [0, 0, 0, 5]);
    assert_eq!(round_trip(GetChat { chat_id: 5 }), [0, 0, 0, 5]);
    assert_eq!(round_trip(GetChatDccAddresses { chat_id: 5 }), [0, 0, 0, 5]);
    assert_eq!(
        round_trip(JoinChat { chat_id: 1, key: 2 }),
        [0, 0, 0, 1, 0, 0, 0, 2]
    );
    assert_eq!(
        round_trip(ChatDeleted {
            chat_id: 1,
            user_id: 2
        }),
        [0, 0, 0, 1, 0, 0, 0, 2]
    );
    assert!(DeleteChat::decode(&[0, 0, 0]).is_err());
}

#[test]
fn chat_window_info_strings_follow_the_members_in_their_own_order() {
    let bytes = round_trip(window_info());
    // 0x24 + 4 member bytes, then name, topic, setter, irc server, irc channel
    assert_eq!(bytes.len(), 0x24 + 4 + 11 + 9 + 7 + 10 + 2);
    assert_eq!(bytes[0x1e], 10, "irc_server_len at 0x1e");
    assert_eq!(bytes[0x20], 11, "name_len at 0x20");
    assert_eq!(&bytes[0x28..0x28 + 11], b"Public Chat");
    assert_eq!(&bytes[bytes.len() - 2..], b"#k");
    rejects_trailing::<ChatWindowInfo>(&bytes);

    let bytes = round_trip(CreatePrivateChatReply {
        info: window_info(),
    });
    assert_eq!(bytes[0x16..0x18], [0, 1]);
}

#[test]
fn chat_user_joined_and_departed() {
    let joined = round_trip(ChatUserJoined {
        chat_id: 1,
        user_id: 2,
        ignored_08: 0,
        ignored_09: [0; 3],
    });
    assert_eq!(joined, [0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 0]);
    let departed = round_trip(ChatUserDeparted {
        chat_id: 1,
        user_id: 2,
        ignored_08: 1,
        ignored_09: [0; 3],
    });
    assert_eq!(departed[8], 1);
}

#[test]
fn chat_text_both_ways() {
    let bytes = round_trip(SendChatText {
        chat_id: 1,
        user_id: 0,
        flags: 0,
        ignored_0a: [0; 2],
        text: b"hello from the lab".to_vec(),
    });
    assert_eq!(bytes.len(), 12 + 18); // 30, as captured
    // `/me waves` goes out as "waves" with the action flag [O chat-basic 24, 25]
    let action = ReceiveChatText {
        chat_id: 1,
        user_id: 1,
        flags: ReceiveChatText::FLAG_ACTION,
        ignored_0a: [0; 2],
        text: b"waves".to_vec(),
    };
    assert!(action.is_action());
    let bytes = round_trip(action);
    assert_eq!(&bytes[8..12], &[0, 1, 0, 0]);
    assert_eq!(&bytes[12..], b"waves");
    assert!(SendChatText::decode(&[0; 11]).is_err());
}

#[test]
fn topics() {
    let bytes = round_trip(SetChatTopic {
        chat_id: 1,
        ignored_04: [0; 6],
        ignored_0b: 0,
        topic: b"lab topic".to_vec(),
        rest: vec![],
    });
    assert_eq!(bytes.len(), 12 + 9); // 21, as captured
    let bytes = round_trip(ChatTopicChanged {
        time: Time1900(1),
        chat_id: 1,
        user_id: 1,
        ignored_10: [0; 8],
        ignored_1a: [0; 2],
        topic: b"lab topic".to_vec(),
        nick: b"labuser".to_vec(),
    });
    assert_eq!(bytes.len(), 0x1c + 9 + 7); // 44, as captured
}

#[test]
fn invites() {
    let bytes = round_trip(ChatInvite {
        chat_id: 1,
        ignored_04: [0; 4],
        ignored_0a: [0; 5],
        user_ids: vec![1, 4],
        message: b"Private 1".to_vec(),
    });
    assert_eq!(bytes.len(), 0x10 + 8 + 9);
    let bytes = round_trip(ChatInvitation {
        chat_id: 1,
        key: 2,
        user_id: 3,
        ignored_0c: 1,
        ignored_10: 1,
        ignored_12: [0; 6],
        ignored_1a: [0; 2],
        message: vec![],
        name: b"Private 1".to_vec(),
    });
    assert_eq!(bytes.len(), 37); // as in the reference captures
}

#[test]
fn chat_list_entries_are_padded() {
    let entry = |id: u32, name: &[u8], desc: &[u8], pad: &[u8]| ChatListEntry {
        chat_id: id,
        key: 0,
        ignored: [0; 6],
        flags: 0,
        member_count: 0,
        name: name.to_vec(),
        description: desc.to_vec(),
        padding: pad.to_vec(),
    };
    // 0x14 + 11 + 13 = 44 → no padding; 0x14 + 10 + 5 = 35 → 1 byte
    let list = ChatList {
        chats: vec![
            entry(1, b"Public Chat", b"For all users", &[]),
            entry(2, b"Voice Chat", b"voice", &[0]),
        ],
    };
    let bytes = round_trip(list);
    assert_eq!(bytes.len(), 4 + 44 + 36);
    assert_eq!(ChatList::decode(&[0, 0, 0, 0]).unwrap().chats.len(), 0);
}

#[test]
fn chat_records_in_create_info_and_set() {
    round_trip(CreateChat {
        chat: chat_record(),
    });
    round_trip(ChatInfo {
        chat: chat_record(),
    });
    let bytes = round_trip(SetChat {
        chat: ChatRecord {
            flags: 2,
            ..chat_record()
        },
    });
    rejects_trailing::<SetChat>(&bytes);
}

#[test]
fn dcc_addresses() {
    assert_eq!(
        round_trip(ChatDccAddresses { addresses: vec![] }),
        Vec::<u8>::new()
    );
    let bytes = round_trip(ChatDccAddresses {
        addresses: vec![DccAddress {
            user_id: 1,
            ignored_04: [0; 10],
            ignored_0f: 0,
            address: b"127.0.0.1:10600".to_vec(),
            padding: vec![0],
        }],
    });
    assert_eq!(bytes.len(), 0x10 + 15 + 1);
}

#[test]
fn private_messages() {
    let bytes = round_trip(SendPrivateMessage {
        user_id: 2,
        ignored: [0; 12],
        text: b"hi from admin".to_vec(),
    });
    assert_eq!(bytes.len(), 16 + 13); // 29, as captured
    let bytes = round_trip(IncomingPrivateMessage {
        user_id: 1,
        ignored: [0; 12],
        text: b"hello back".to_vec(),
    });
    assert_eq!(bytes.len(), 26);
    assert!(
        SendPrivateMessage::decode(&[0; 16]).is_err(),
        "at least 0x11 bytes"
    );
}
