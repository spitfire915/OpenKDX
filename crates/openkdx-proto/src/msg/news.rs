//! Messages 600–614: newsgroups and news messages. See the reference spec's
//! `messages/news.md`.

use crate::message_type::MessageType;
use crate::msg::common::Time1900;
use crate::msg::{Payload, empty_payload};
use crate::wire::{DecodeError, Reader, Writer};

/// A newsgroup, as 600, 603, and 604 carry it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NewsgroupInfo {
    pub group_id: u32,
    pub ignored_04: [u8; 12],
    /// Set by the last 611 Post News Message.
    pub time: Time1900,
    /// Class bitmaps; class 0 is the top bit.
    pub read_classes: u32,
    pub post_classes: u32,
    /// A third class bitmap; the server sends `01 00 00 00`, the client 0.
    pub ignored_20: [u8; 4],
    pub ignored_24: [u8; 12],
    pub message_count: u16,
    /// Archive size in KiB, 4 to 1024; 50 by default.
    pub size_limit_kib: u16,
    pub ignored_34: [u8; 8],
    pub ignored_3e: [u8; 2],
    /// 1 to 0x3f bytes.
    pub name: Vec<u8>,
    /// At most 0x7f bytes.
    pub description: Vec<u8>,
}

impl NewsgroupInfo {
    pub fn decode(r: &mut Reader) -> Result<Self, DecodeError> {
        let group_id = r.u32()?;
        let ignored_04: [u8; 12] = r.array()?;
        let time = Time1900::decode(r)?;
        let read_classes = r.u32()?;
        let post_classes = r.u32()?;
        let ignored_20: [u8; 4] = r.array()?;
        let ignored_24: [u8; 12] = r.array()?;
        let message_count = r.u16()?;
        let size_limit_kib = r.u16()?;
        let ignored_34: [u8; 8] = r.array()?;
        let name_len = usize::from(r.u8()?);
        let description_len = usize::from(r.u8()?);
        let ignored_3e: [u8; 2] = r.array()?;
        Ok(NewsgroupInfo {
            group_id,
            ignored_04,
            time,
            read_classes,
            post_classes,
            ignored_20,
            ignored_24,
            message_count,
            size_limit_kib,
            ignored_34,
            ignored_3e,
            name: r.bytes(name_len)?,
            description: r.bytes(description_len)?,
        })
    }

    pub fn encode(&self, w: &mut Writer) {
        w.u32(self.group_id);
        w.bytes(&self.ignored_04);
        self.time.encode(w);
        w.u32(self.read_classes);
        w.u32(self.post_classes);
        w.bytes(&self.ignored_20);
        w.bytes(&self.ignored_24);
        w.u16(self.message_count);
        w.u16(self.size_limit_kib);
        w.bytes(&self.ignored_34);
        w.u8(self.name.len() as u8);
        w.u8(self.description.len() as u8);
        w.bytes(&self.ignored_3e);
        w.bytes(&self.name);
        w.bytes(&self.description);
    }
}

macro_rules! newsgroup_info_payload {
    ($(#[$doc:meta] $name:ident = $kind:ident;)*) => {
        $(
            #[$doc]
            #[derive(Debug, Clone, PartialEq, Eq, Default)]
            pub struct $name {
                pub group: NewsgroupInfo,
            }

            impl Payload for $name {
                const KIND: MessageType = MessageType::$kind;

                fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
                    let mut r = Reader::new(bytes);
                    let group = NewsgroupInfo::decode(&mut r)?;
                    r.finish()?;
                    Ok($name { group })
                }

                fn encode(&self, w: &mut Writer) {
                    self.group.encode(w);
                }
            }
        )*
    };
}

newsgroup_info_payload! {
    /// 600: reply is 1 Status 0x258.
    CreateNewsgroup = CREATE_NEWSGROUP;
    /// 603: reply to 602.
    NewsgroupInfoReply = NEWSGROUP_INFO;
    /// 604: reply is 1 Status 0x25c.
    SetNewsgroup = SET_NEWSGROUP;
}

macro_rules! group_id_payload {
    ($(#[$doc:meta] $name:ident = $kind:ident;)*) => {
        $(
            #[$doc]
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
            pub struct $name {
                pub group_id: u32,
            }

            impl Payload for $name {
                const KIND: MessageType = MessageType::$kind;

                fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
                    let mut r = Reader::new(bytes);
                    let group_id = r.u32()?;
                    r.finish()?;
                    Ok($name { group_id })
                }

                fn encode(&self, w: &mut Writer) {
                    w.u32(self.group_id);
                }
            }
        )*
    };
}

group_id_payload! {
    /// 601
    DeleteNewsgroup = DELETE_NEWSGROUP;
    /// 602: reply is 603.
    GetNewsgroup = GET_NEWSGROUP;
    /// 607: reply is 608.
    GetNewsMessages = GET_NEWS_MESSAGES;
}

macro_rules! group_message_payload {
    ($(#[$doc:meta] $name:ident = $kind:ident;)*) => {
        $(
            #[$doc]
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
            pub struct $name {
                pub group_id: u32,
                pub message_id: u32,
            }

            impl Payload for $name {
                const KIND: MessageType = MessageType::$kind;

                fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
                    let mut r = Reader::new(bytes);
                    let out = $name { group_id: r.u32()?, message_id: r.u32()? };
                    r.finish()?;
                    Ok(out)
                }

                fn encode(&self, w: &mut Writer) {
                    w.u32(self.group_id);
                    w.u32(self.message_id);
                }
            }
        )*
    };
}

group_message_payload! {
    /// 609: reply is 610.
    GetNewsMessage = GET_NEWS_MESSAGE;
    /// 612: no reply; all users get 614.
    DeleteNewsMessage = DELETE_NEWS_MESSAGE;
}

empty_payload! {
    /// 605: reply is 606.
    GetNewsgroupList = GET_NEWSGROUP_LIST;
}

/// One group in 606.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NewsgroupEntry {
    pub time: Time1900,
    pub group_id: u32,
    pub ignored_0c: [u8; 4],
    pub message_count: u16,
    pub ignored_12: [u8; 4],
    pub name: Vec<u8>,
    pub description: Vec<u8>,
    pub padding: Vec<u8>,
}

/// 606: reply to 605.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NewsgroupList {
    pub groups: Vec<NewsgroupEntry>,
}

impl Payload for NewsgroupList {
    const KIND: MessageType = MessageType::NEWSGROUP_LIST;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let count = r.u32()? as usize;
        let mut groups = Vec::with_capacity(count.min(1024));
        for _ in 0..count {
            let start = r.pos();
            let time = Time1900::decode(&mut r)?;
            let group_id = r.u32()?;
            let ignored_0c: [u8; 4] = r.array()?;
            let message_count = r.u16()?;
            let ignored_12: [u8; 4] = r.array()?;
            let name_len = usize::from(r.u8()?);
            let description_len = usize::from(r.u8()?);
            groups.push(NewsgroupEntry {
                time,
                group_id,
                ignored_0c,
                message_count,
                ignored_12,
                name: r.bytes(name_len)?,
                description: r.bytes(description_len)?,
                padding: r.pad4(start)?,
            });
        }
        r.finish()?;
        Ok(NewsgroupList { groups })
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.groups.len() as u32);
        for g in &self.groups {
            let start = w.pos();
            g.time.encode(w);
            w.u32(g.group_id);
            w.bytes(&g.ignored_0c);
            w.u16(g.message_count);
            w.bytes(&g.ignored_12);
            w.u8(g.name.len() as u8);
            w.u8(g.description.len() as u8);
            w.bytes(&g.name);
            w.bytes(&g.description);
            w.pad4(start, &g.padding);
        }
    }
}

/// One message, as 608 lists them and 610 returns one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NewsMessage {
    pub time: Time1900,
    pub group_id: u32,
    pub message_id: u32,
    pub ignored_10: [u8; 8],
    pub ignored_1a: [u8; 2],
    pub ignored_1f: u8,
    /// The poster's nick.
    pub poster: Vec<u8>,
    /// The poster's account name.
    pub login: Vec<u8>,
    pub subject: Vec<u8>,
    /// Lines end in CR.
    pub body: Vec<u8>,
    pub padding: Vec<u8>,
}

impl NewsMessage {
    pub fn decode(r: &mut Reader) -> Result<Self, DecodeError> {
        let start = r.pos();
        let time = Time1900::decode(r)?;
        let group_id = r.u32()?;
        let message_id = r.u32()?;
        let ignored_10: [u8; 8] = r.array()?;
        let body_len = usize::from(r.u16()?);
        let ignored_1a: [u8; 2] = r.array()?;
        let poster_len = usize::from(r.u8()?);
        let login_len = usize::from(r.u8()?);
        let subject_len = usize::from(r.u8()?);
        let ignored_1f = r.u8()?;
        Ok(NewsMessage {
            time,
            group_id,
            message_id,
            ignored_10,
            ignored_1a,
            ignored_1f,
            poster: r.bytes(poster_len)?,
            login: r.bytes(login_len)?,
            subject: r.bytes(subject_len)?,
            body: r.bytes(body_len)?,
            padding: r.pad4_lenient(start)?,
        })
    }

    pub fn encode(&self, w: &mut Writer) {
        let start = w.pos();
        self.time.encode(w);
        w.u32(self.group_id);
        w.u32(self.message_id);
        w.bytes(&self.ignored_10);
        w.u16(self.body.len() as u16);
        w.bytes(&self.ignored_1a);
        w.u8(self.poster.len() as u8);
        w.u8(self.login.len() as u8);
        w.u8(self.subject.len() as u8);
        w.u8(self.ignored_1f);
        w.bytes(&self.poster);
        w.bytes(&self.login);
        w.bytes(&self.subject);
        w.bytes(&self.body);
        if !self.padding.is_empty() {
            w.pad4(start, &self.padding);
        }
    }
}

/// 608: reply to 607.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NewsMessages {
    pub group_id: u32,
    /// 1 post, 2 delete any message, 4 read.
    pub permissions: u32,
    pub ignored: [u8; 4],
    pub messages: Vec<NewsMessage>,
}

impl NewsMessages {
    pub const PERMISSION_POST: u32 = 1;
    pub const PERMISSION_DELETE: u32 = 2;
    pub const PERMISSION_READ: u32 = 4;
}

impl Payload for NewsMessages {
    const KIND: MessageType = MessageType::NEWS_MESSAGES;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let group_id = r.u32()?;
        let permissions = r.u32()?;
        let ignored: [u8; 4] = r.array()?;
        let count = r.u32()? as usize;
        let mut messages = Vec::with_capacity(count.min(4096));
        for _ in 0..count {
            messages.push(NewsMessage::decode(&mut r)?);
        }
        r.finish()?;
        Ok(NewsMessages {
            group_id,
            permissions,
            ignored,
            messages,
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.group_id);
        w.u32(self.permissions);
        w.bytes(&self.ignored);
        w.u32(self.messages.len() as u32);
        for m in &self.messages {
            m.encode(w);
        }
    }
}

/// 610: reply to 609.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NewsMessageReply {
    pub message: NewsMessage,
}

impl Payload for NewsMessageReply {
    const KIND: MessageType = MessageType::NEWS_MESSAGE;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let message = NewsMessage::decode(&mut r)?;
        r.finish()?;
        Ok(NewsMessageReply { message })
    }

    fn encode(&self, w: &mut Writer) {
        self.message.encode(w);
    }
}

/// 611: no reply; readers of the group get 613.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PostNewsMessage {
    pub group_id: u32,
    pub ignored_04: [u8; 8],
    pub ignored_0f: u8,
    /// The server keeps at most 0x80 bytes.
    pub subject: Vec<u8>,
    /// Lines end in CR; the server keeps at most 0x4000 bytes.
    pub body: Vec<u8>,
    /// Any bytes after the body.
    pub rest: Vec<u8>,
}

impl Payload for PostNewsMessage {
    const KIND: MessageType = MessageType::POST_NEWS_MESSAGE;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let group_id = r.u32()?;
        let ignored_04: [u8; 8] = r.array()?;
        let body_len = usize::from(r.u16()?);
        let subject_len = usize::from(r.u8()?);
        let ignored_0f = r.u8()?;
        Ok(PostNewsMessage {
            group_id,
            ignored_04,
            ignored_0f,
            subject: r.bytes(subject_len)?,
            body: r.bytes(body_len)?,
            rest: r.rest(),
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.group_id);
        w.bytes(&self.ignored_04);
        w.u16(self.body.len() as u16);
        w.u8(self.subject.len() as u8);
        w.u8(self.ignored_0f);
        w.bytes(&self.subject);
        w.bytes(&self.body);
        w.bytes(&self.rest);
    }
}

macro_rules! news_change_payload {
    ($(#[$doc:meta] $name:ident = $kind:ident;)*) => {
        $(
            #[$doc]
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
            pub struct $name {
                pub group_id: u32,
                pub message_id: u32,
                /// Messages in the group after the change.
                pub message_count: u32,
            }

            impl Payload for $name {
                const KIND: MessageType = MessageType::$kind;

                fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
                    let mut r = Reader::new(bytes);
                    let out = $name {
                        group_id: r.u32()?,
                        message_id: r.u32()?,
                        message_count: r.u32()?,
                    };
                    r.finish()?;
                    Ok(out)
                }

                fn encode(&self, w: &mut Writer) {
                    w.u32(self.group_id);
                    w.u32(self.message_id);
                    w.u32(self.message_count);
                }
            }
        )*
    };
}

news_change_payload! {
    /// 613: after a successful 611.
    NewsMessagePosted = NEWS_MESSAGE_POSTED;
    /// 614: after a successful 612.
    NewsMessageDeleted = NEWS_MESSAGE_DELETED;
}
