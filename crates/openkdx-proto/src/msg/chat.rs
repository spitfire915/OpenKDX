//! Messages 200–223 (chat rooms) and 300–301 (private messages). See the
//! reference spec's `messages/chat.md` and `messages/private-messages.md`.

use crate::message_type::MessageType;
use crate::msg::common::{ChatRecord, Time1900};
use crate::msg::{Payload, empty_payload};
use crate::wire::{DecodeError, Reader, Writer};

/// A message whose payload is one u32 chat ID.
macro_rules! chat_id_payload {
    ($(#[$doc:meta] $name:ident = $kind:ident;)*) => {
        $(
            #[$doc]
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
            pub struct $name {
                pub chat_id: u32,
            }

            impl Payload for $name {
                const KIND: MessageType = MessageType::$kind;

                fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
                    let mut r = Reader::new(bytes);
                    let chat_id = r.u32()?;
                    r.finish()?;
                    Ok($name { chat_id })
                }

                fn encode(&self, w: &mut Writer) {
                    w.u32(self.chat_id);
                }
            }
        )*
    };
}

/// A message whose payload is one chat record.
macro_rules! chat_record_payload {
    ($(#[$doc:meta] $name:ident = $kind:ident;)*) => {
        $(
            #[$doc]
            #[derive(Debug, Clone, PartialEq, Eq)]
            pub struct $name {
                pub chat: ChatRecord,
            }

            impl Payload for $name {
                const KIND: MessageType = MessageType::$kind;

                fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
                    let mut r = Reader::new(bytes);
                    let chat = ChatRecord::decode(&mut r)?;
                    r.finish()?;
                    Ok($name { chat })
                }

                fn encode(&self, w: &mut Writer) {
                    self.chat.encode(w);
                }
            }
        )*
    };
}

empty_payload! {
    /// 200: reply is 216.
    CreatePrivateChat = CREATE_PRIVATE_CHAT;
    /// 213: sent after login when Join Chat On Connect is set.
    MustJoinChat = MUST_JOIN_CHAT;
    /// 214: reply is 215.
    GetChatList = GET_CHAT_LIST;
}

chat_id_payload! {
    /// 201
    DeleteChat = DELETE_CHAT;
    /// 204
    LeaveChat = LEAVE_CHAT;
    /// 218: reply is 219.
    GetChat = GET_CHAT;
    /// 222: reply is 223.
    GetChatDccAddresses = GET_CHAT_DCC_ADDRESSES;
}

chat_record_payload! {
    /// 217: `chat_id` is 0.
    CreateChat = CREATE_CHAT;
    /// 219: reply to 218.
    ChatInfo = CHAT_INFO;
    /// 220
    SetChat = SET_CHAT;
}

/// 202: reply is 203.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct JoinChat {
    pub chat_id: u32,
    /// Needed for private chats.
    pub key: u32,
}

impl Payload for JoinChat {
    const KIND: MessageType = MessageType::JOIN_CHAT;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let out = JoinChat {
            chat_id: r.u32()?,
            key: r.u32()?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.chat_id);
        w.u32(self.key);
    }
}

/// 203: reply to 202. The five strings follow the member IDs in the order
/// name, topic, topic setter, IRC server, IRC channel; their lengths are
/// stored in a different order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatWindowInfo {
    pub time: Time1900,
    pub chat_id: u32,
    pub key: u32,
    pub ignored_10: [u8; 4],
    /// Bit 0 private, bit 1 voice.
    pub flags: u16,
    pub ignored_18: [u8; 6],
    pub ignored_23: u8,
    pub members: Vec<u32>,
    pub name: Vec<u8>,
    pub topic: Vec<u8>,
    pub topic_setter: Vec<u8>,
    pub irc_server: Vec<u8>,
    pub irc_channel: Vec<u8>,
}

impl ChatWindowInfo {
    fn decode_from(r: &mut Reader) -> Result<Self, DecodeError> {
        let time = Time1900::decode(r)?;
        let chat_id = r.u32()?;
        let key = r.u32()?;
        let ignored_10: [u8; 4] = r.array()?;
        let flags = r.u16()?;
        let members_count = usize::from(r.u16()?);
        let ignored_18: [u8; 6] = r.array()?;
        let irc_server_len = usize::from(r.u8()?);
        let irc_channel_len = usize::from(r.u8()?);
        let name_len = usize::from(r.u8()?);
        let topic_len = usize::from(r.u8()?);
        let topic_setter_len = usize::from(r.u8()?);
        let ignored_23 = r.u8()?;
        let mut members = Vec::with_capacity(members_count.min(1024));
        for _ in 0..members_count {
            members.push(r.u32()?);
        }
        Ok(ChatWindowInfo {
            time,
            chat_id,
            key,
            ignored_10,
            flags,
            ignored_18,
            ignored_23,
            members,
            name: r.bytes(name_len)?,
            topic: r.bytes(topic_len)?,
            topic_setter: r.bytes(topic_setter_len)?,
            irc_server: r.bytes(irc_server_len)?,
            irc_channel: r.bytes(irc_channel_len)?,
        })
    }

    fn encode_into(&self, w: &mut Writer) {
        self.time.encode(w);
        w.u32(self.chat_id);
        w.u32(self.key);
        w.bytes(&self.ignored_10);
        w.u16(self.flags);
        w.u16(self.members.len() as u16);
        w.bytes(&self.ignored_18);
        w.u8(self.irc_server.len() as u8);
        w.u8(self.irc_channel.len() as u8);
        w.u8(self.name.len() as u8);
        w.u8(self.topic.len() as u8);
        w.u8(self.topic_setter.len() as u8);
        w.u8(self.ignored_23);
        for m in &self.members {
            w.u32(*m);
        }
        w.bytes(&self.name);
        w.bytes(&self.topic);
        w.bytes(&self.topic_setter);
        w.bytes(&self.irc_server);
        w.bytes(&self.irc_channel);
    }
}

impl Payload for ChatWindowInfo {
    const KIND: MessageType = MessageType::CHAT_WINDOW_INFO;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let out = Self::decode_from(&mut r)?;
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        self.encode_into(w);
    }
}

/// 216: reply to 200, with the layout of 203 and the creator as the one member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreatePrivateChatReply {
    pub info: ChatWindowInfo,
}

impl Payload for CreatePrivateChatReply {
    const KIND: MessageType = MessageType::CREATE_PRIVATE_CHAT_REPLY;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let info = ChatWindowInfo::decode_from(&mut r)?;
        r.finish()?;
        Ok(CreatePrivateChatReply { info })
    }

    fn encode(&self, w: &mut Writer) {
        self.info.encode_into(w);
    }
}

/// 205 and 206 share this 12-byte layout.
macro_rules! chat_user_change {
    ($(#[$doc:meta] $name:ident = $kind:ident;)*) => {
        $(
            #[$doc]
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
            pub struct $name {
                pub chat_id: u32,
                pub user_id: u32,
                /// 0 when joining, 1 when leaving.
                pub ignored_08: u8,
                pub ignored_09: [u8; 3],
            }

            impl Payload for $name {
                const KIND: MessageType = MessageType::$kind;

                fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
                    let mut r = Reader::new(bytes);
                    let out = $name {
                        chat_id: r.u32()?,
                        user_id: r.u32()?,
                        ignored_08: r.u8()?,
                        ignored_09: r.array()?,
                    };
                    r.finish()?;
                    Ok(out)
                }

                fn encode(&self, w: &mut Writer) {
                    w.u32(self.chat_id);
                    w.u32(self.user_id);
                    w.u8(self.ignored_08);
                    w.bytes(&self.ignored_09);
                }
            }
        )*
    };
}

chat_user_change! {
    /// 205: a member joined.
    ChatUserJoined = CHAT_USER_JOINED;
    /// 206: a member left; byte 0x08 is 1.
    ChatUserDeparted = CHAT_USER_DEPARTED;
}

/// 207 and 208 share this layout.
macro_rules! chat_text {
    ($(#[$doc:meta] $name:ident = $kind:ident;)*) => {
        $(
            #[$doc]
            #[derive(Debug, Clone, PartialEq, Eq, Default)]
            pub struct $name {
                pub chat_id: u32,
                /// 0 from the client; the server fills in the sender.
                pub user_id: u32,
                /// Bit 0: an action (`/me`); the client strips the `/me ` and
                /// sets the bit. The reference spec marks these bytes ignored;
                /// see docs/protocol/README.md.
                pub flags: u16,
                pub ignored_0a: [u8; 2],
                /// At most 0x1000 bytes; the server cuts longer text.
                pub text: Vec<u8>,
            }

            impl $name {
                pub const FLAG_ACTION: u16 = 1;

                pub fn is_action(&self) -> bool {
                    self.flags & Self::FLAG_ACTION != 0
                }
            }

            impl Payload for $name {
                const KIND: MessageType = MessageType::$kind;

                fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
                    let mut r = Reader::new(bytes);
                    Ok($name {
                        chat_id: r.u32()?,
                        user_id: r.u32()?,
                        flags: r.u16()?,
                        ignored_0a: r.array()?,
                        text: r.rest(),
                    })
                }

                fn encode(&self, w: &mut Writer) {
                    w.u32(self.chat_id);
                    w.u32(self.user_id);
                    w.u16(self.flags);
                    w.bytes(&self.ignored_0a);
                    w.bytes(&self.text);
                }
            }
        )*
    };
}

chat_text! {
    /// 207
    SendChatText = SEND_CHAT_TEXT;
    /// 208: forwarded from 207 with the sender's ID.
    ReceiveChatText = RECEIVE_CHAT_TEXT;
}

impl SendChatText {
    pub const MAX_TEXT: usize = 0x1000;
}

/// 209
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SetChatTopic {
    pub chat_id: u32,
    pub ignored_04: [u8; 6],
    pub ignored_0b: u8,
    pub topic: Vec<u8>,
    /// Bytes after the topic; the server ignores them.
    pub rest: Vec<u8>,
}

impl Payload for SetChatTopic {
    const KIND: MessageType = MessageType::SET_CHAT_TOPIC;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let chat_id = r.u32()?;
        let ignored_04: [u8; 6] = r.array()?;
        let topic_len = usize::from(r.u8()?);
        let ignored_0b = r.u8()?;
        Ok(SetChatTopic {
            chat_id,
            ignored_04,
            ignored_0b,
            topic: r.bytes(topic_len)?,
            rest: r.rest(),
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.chat_id);
        w.bytes(&self.ignored_04);
        w.u8(self.topic.len() as u8);
        w.u8(self.ignored_0b);
        w.bytes(&self.topic);
        w.bytes(&self.rest);
    }
}

/// 210
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChatTopicChanged {
    pub time: Time1900,
    pub chat_id: u32,
    pub user_id: u32,
    pub ignored_10: [u8; 8],
    pub ignored_1a: [u8; 2],
    pub topic: Vec<u8>,
    pub nick: Vec<u8>,
}

impl Payload for ChatTopicChanged {
    const KIND: MessageType = MessageType::CHAT_TOPIC_CHANGED;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let time = Time1900::decode(&mut r)?;
        let chat_id = r.u32()?;
        let user_id = r.u32()?;
        let ignored_10: [u8; 8] = r.array()?;
        let topic_len = usize::from(r.u8()?);
        let nick_len = usize::from(r.u8()?);
        let ignored_1a: [u8; 2] = r.array()?;
        let out = ChatTopicChanged {
            time,
            chat_id,
            user_id,
            ignored_10,
            ignored_1a,
            topic: r.bytes(topic_len)?,
            nick: r.bytes(nick_len)?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        self.time.encode(w);
        w.u32(self.chat_id);
        w.u32(self.user_id);
        w.bytes(&self.ignored_10);
        w.u8(self.topic.len() as u8);
        w.u8(self.nick.len() as u8);
        w.bytes(&self.ignored_1a);
        w.bytes(&self.topic);
        w.bytes(&self.nick);
    }
}

/// 211
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChatInvite {
    pub chat_id: u32,
    pub ignored_04: [u8; 4],
    pub ignored_0a: [u8; 5],
    /// At most 20.
    pub user_ids: Vec<u32>,
    pub message: Vec<u8>,
}

impl Payload for ChatInvite {
    const KIND: MessageType = MessageType::CHAT_INVITE;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let chat_id = r.u32()?;
        let ignored_04: [u8; 4] = r.array()?;
        let count = usize::from(r.u16()?);
        let ignored_0a: [u8; 5] = r.array()?;
        let message_len = usize::from(r.u8()?);
        let mut user_ids = Vec::with_capacity(count.min(64));
        for _ in 0..count {
            user_ids.push(r.u32()?);
        }
        let out = ChatInvite {
            chat_id,
            ignored_04,
            ignored_0a,
            user_ids,
            message: r.bytes(message_len)?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.chat_id);
        w.bytes(&self.ignored_04);
        w.u16(self.user_ids.len() as u16);
        w.bytes(&self.ignored_0a);
        w.u8(self.message.len() as u8);
        for id in &self.user_ids {
            w.u32(*id);
        }
        w.bytes(&self.message);
    }
}

/// 212
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChatInvitation {
    pub chat_id: u32,
    /// For 202 Join Chat.
    pub key: u32,
    pub user_id: u32,
    pub ignored_0c: u32,
    pub ignored_10: u16,
    pub ignored_12: [u8; 6],
    pub ignored_1a: [u8; 2],
    pub message: Vec<u8>,
    pub name: Vec<u8>,
}

impl Payload for ChatInvitation {
    const KIND: MessageType = MessageType::CHAT_INVITATION;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let chat_id = r.u32()?;
        let key = r.u32()?;
        let user_id = r.u32()?;
        let ignored_0c = r.u32()?;
        let ignored_10 = r.u16()?;
        let ignored_12: [u8; 6] = r.array()?;
        let message_len = usize::from(r.u8()?);
        let name_len = usize::from(r.u8()?);
        let ignored_1a: [u8; 2] = r.array()?;
        let out = ChatInvitation {
            chat_id,
            key,
            user_id,
            ignored_0c,
            ignored_10,
            ignored_12,
            ignored_1a,
            message: r.bytes(message_len)?,
            name: r.bytes(name_len)?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.chat_id);
        w.u32(self.key);
        w.u32(self.user_id);
        w.u32(self.ignored_0c);
        w.u16(self.ignored_10);
        w.bytes(&self.ignored_12);
        w.u8(self.message.len() as u8);
        w.u8(self.name.len() as u8);
        w.bytes(&self.ignored_1a);
        w.bytes(&self.message);
        w.bytes(&self.name);
    }
}

/// One chat in 215 Chat List.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChatListEntry {
    pub chat_id: u32,
    pub key: u32,
    pub ignored: [u8; 6],
    /// Bit 1 voice.
    pub flags: u16,
    pub member_count: u16,
    pub name: Vec<u8>,
    pub description: Vec<u8>,
    pub padding: Vec<u8>,
}

impl ChatListEntry {
    fn decode(r: &mut Reader) -> Result<Self, DecodeError> {
        let start = r.pos();
        let chat_id = r.u32()?;
        let key = r.u32()?;
        let ignored: [u8; 6] = r.array()?;
        let flags = r.u16()?;
        let member_count = r.u16()?;
        let name_len = usize::from(r.u8()?);
        let description_len = usize::from(r.u8()?);
        Ok(ChatListEntry {
            chat_id,
            key,
            ignored,
            flags,
            member_count,
            name: r.bytes(name_len)?,
            description: r.bytes(description_len)?,
            padding: r.pad4(start)?,
        })
    }

    fn encode(&self, w: &mut Writer) {
        let start = w.pos();
        w.u32(self.chat_id);
        w.u32(self.key);
        w.bytes(&self.ignored);
        w.u16(self.flags);
        w.u16(self.member_count);
        w.u8(self.name.len() as u8);
        w.u8(self.description.len() as u8);
        w.bytes(&self.name);
        w.bytes(&self.description);
        w.pad4(start, &self.padding);
    }
}

/// 215: reply to 214.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChatList {
    pub chats: Vec<ChatListEntry>,
}

impl Payload for ChatList {
    const KIND: MessageType = MessageType::CHAT_LIST;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let count = r.u32()? as usize;
        let mut chats = Vec::with_capacity(count.min(1024));
        for _ in 0..count {
            chats.push(ChatListEntry::decode(&mut r)?);
        }
        r.finish()?;
        Ok(ChatList { chats })
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.chats.len() as u32);
        for c in &self.chats {
            c.encode(w);
        }
    }
}

/// 221
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ChatDeleted {
    pub chat_id: u32,
    /// The requester.
    pub user_id: u32,
}

impl Payload for ChatDeleted {
    const KIND: MessageType = MessageType::CHAT_DELETED;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let out = ChatDeleted {
            chat_id: r.u32()?,
            user_id: r.u32()?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.chat_id);
        w.u32(self.user_id);
    }
}

/// One member's DCC address in 223.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DccAddress {
    pub user_id: u32,
    pub ignored_04: [u8; 10],
    pub ignored_0f: u8,
    /// `ip:port`.
    pub address: Vec<u8>,
    pub padding: Vec<u8>,
}

/// 223: reply to 222. One entry per other member with a DCC port.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChatDccAddresses {
    pub addresses: Vec<DccAddress>,
}

impl Payload for ChatDccAddresses {
    const KIND: MessageType = MessageType::CHAT_DCC_ADDRESSES;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let mut addresses = Vec::new();
        while !r.is_empty() {
            let start = r.pos();
            let user_id = r.u32()?;
            let ignored_04: [u8; 10] = r.array()?;
            let address_len = usize::from(r.u8()?);
            let ignored_0f = r.u8()?;
            addresses.push(DccAddress {
                user_id,
                ignored_04,
                ignored_0f,
                address: r.bytes(address_len)?,
                padding: r.pad4(start)?,
            });
        }
        Ok(ChatDccAddresses { addresses })
    }

    fn encode(&self, w: &mut Writer) {
        for a in &self.addresses {
            let start = w.pos();
            w.u32(a.user_id);
            w.bytes(&a.ignored_04);
            w.u8(a.address.len() as u8);
            w.u8(a.ignored_0f);
            w.bytes(&a.address);
            w.pad4(start, &a.padding);
        }
    }
}

/// 300 and 301 share this layout; at least 0x11 bytes.
macro_rules! private_message {
    ($(#[$doc:meta] $name:ident = $kind:ident;)*) => {
        $(
            #[$doc]
            #[derive(Debug, Clone, PartialEq, Eq, Default)]
            pub struct $name {
                /// The target in 300, the sender in 301.
                pub user_id: u32,
                pub ignored: [u8; 12],
                pub text: Vec<u8>,
            }

            impl Payload for $name {
                const KIND: MessageType = MessageType::$kind;

                fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
                    let mut r = Reader::new(bytes);
                    let user_id = r.u32()?;
                    let ignored: [u8; 12] = r.array()?;
                    let text = r.rest();
                    if text.is_empty() {
                        return Err(DecodeError::Invalid("private message shorter than 0x11 bytes"));
                    }
                    Ok($name { user_id, ignored, text })
                }

                fn encode(&self, w: &mut Writer) {
                    w.u32(self.user_id);
                    w.bytes(&self.ignored);
                    w.bytes(&self.text);
                }
            }
        )*
    };
}

private_message! {
    /// 300
    SendPrivateMessage = SEND_PRIVATE_MESSAGE;
    /// 301: forwarded from 300 with the sender's ID.
    IncomingPrivateMessage = INCOMING_PRIVATE_MESSAGE;
}
