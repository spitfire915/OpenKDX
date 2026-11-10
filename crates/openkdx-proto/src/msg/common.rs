//! Records that many messages share: colors, times, paths, users, chats.
//! See the reference spec's `encodings.md`.

use std::fmt;

use crate::wire::{DecodeError, Reader, Writer};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    pub fn decode(r: &mut Reader) -> Result<Self, DecodeError> {
        Ok(Color {
            r: r.u8()?,
            g: r.u8()?,
            b: r.u8()?,
        })
    }

    pub fn encode(&self, w: &mut Writer) {
        w.u8(self.r);
        w.u8(self.g);
        w.u8(self.b);
    }
}

/// Milliseconds since 1900-01-01 00:00 UTC.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Time1900(pub u64);

impl Time1900 {
    pub const UNIX_EPOCH_OFFSET_MS: i64 = 2_208_988_800_000;

    pub fn to_unix_ms(self) -> i64 {
        self.0 as i64 - Self::UNIX_EPOCH_OFFSET_MS
    }

    pub fn from_unix_ms(ms: i64) -> Self {
        Time1900((ms + Self::UNIX_EPOCH_OFFSET_MS) as u64)
    }

    pub fn decode(r: &mut Reader) -> Result<Self, DecodeError> {
        Ok(Time1900(r.u64()?))
    }

    pub fn encode(self, w: &mut Writer) {
        w.u64(self.0);
    }
}

/// One level of a [`FilePath`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathName {
    pub ignored: u16,
    pub name: Vec<u8>,
}

impl PathName {
    pub fn new(name: &[u8]) -> Self {
        PathName {
            ignored: 0,
            name: name.to_vec(),
        }
    }
}

/// A path below the user's base folder: a list of names.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FilePath {
    pub names: Vec<PathName>,
}

impl FilePath {
    pub const MAX_NAMES: usize = 0x400;

    pub fn root() -> Self {
        Self::default()
    }

    pub fn is_root(&self) -> bool {
        self.names.is_empty()
    }

    /// A whole payload: the root is empty, and a count of 0 is malformed.
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        if bytes.is_empty() {
            return Ok(Self::root());
        }
        let mut r = Reader::new(bytes);
        let path = Self::decode_names(&mut r, 1)?;
        r.finish()?;
        Ok(path)
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut w = Writer::new();
        if !self.is_root() {
            self.encode_names(&mut w);
        }
        w.into_inner()
    }

    /// A path followed by more fields: the root is `00 00`.
    pub fn decode_delimited(r: &mut Reader) -> Result<Self, DecodeError> {
        Self::decode_names(r, 0)
    }

    pub fn encode_delimited(&self, w: &mut Writer) {
        self.encode_names(w);
    }

    fn decode_names(r: &mut Reader, min: usize) -> Result<Self, DecodeError> {
        let count = usize::from(r.u16()?);
        if count < min || count > Self::MAX_NAMES {
            return Err(DecodeError::Invalid("file path name count out of range"));
        }
        let mut names = Vec::with_capacity(count);
        for _ in 0..count {
            let ignored = r.u16()?;
            let len = usize::from(r.u8()?);
            if len == 0 {
                return Err(DecodeError::Invalid("empty file path name"));
            }
            names.push(PathName {
                ignored,
                name: r.bytes(len)?,
            });
        }
        Ok(FilePath { names })
    }

    fn encode_names(&self, w: &mut Writer) {
        w.u16(self.names.len() as u16);
        for n in &self.names {
            w.u16(n.ignored);
            w.u8(n.name.len().min(0xff) as u8);
            w.bytes(&n.name[..n.name.len().min(0xff)]);
        }
    }
}

impl fmt::Display for FilePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, n) in self.names.iter().enumerate() {
            if i > 0 {
                f.write_str("/")?;
            }
            f.write_str(&String::from_utf8_lossy(&n.name))?;
        }
        Ok(())
    }
}

/// A user, as 100 Set Identity, 101 User List, 102 User Changed, and
/// 103 User Joined carry it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserRecord {
    /// 0 from the client in 100 Set Identity.
    pub user_id: u32,
    pub status: u8,
    pub ignored_05: [u8; 5],
    /// The client's DCC listener port, 10600 by default, or 0.
    pub client_port: u16,
    pub text_color: Color,
    pub background_color: Color,
    /// Set by the server from the class or account colour.
    pub account_color: Color,
    /// Bit 2 (from the client): gradient row. Bit 1 (from the server): hidden marker seen.
    pub flags: u8,
    pub ignored_1b: u8,
    /// 2 bits per pixel bitmap, less than 0x709 bytes.
    pub icon: Vec<u8>,
    pub extra: Vec<u8>,
    pub nick: Vec<u8>,
    pub info: Vec<u8>,
    /// Zero to three bytes to a multiple of 4: the server pads, the client does not.
    pub padding: Vec<u8>,
}

impl UserRecord {
    pub const FLAG_GRADIENT: u8 = 1 << 2;
    pub const FLAG_MARKER: u8 = 1 << 1;

    pub fn decode(r: &mut Reader) -> Result<Self, DecodeError> {
        let start = r.pos();
        let user_id = r.u32()?;
        let status = r.u8()?;
        let ignored_05: [u8; 5] = r.array()?;
        let client_port = r.u16()?;
        let icon_len = usize::from(r.u16()?);
        let text_color = Color::decode(r)?;
        let background_color = Color::decode(r)?;
        let account_color = Color::decode(r)?;
        let flags = r.u8()?;
        let extra_len = usize::from(r.u8()?);
        let nick_len = usize::from(r.u8()?);
        let info_len = usize::from(r.u8()?);
        let ignored_1b = r.u8()?;
        let icon = r.bytes(icon_len)?;
        let extra = r.bytes(extra_len)?;
        let nick = r.bytes(nick_len)?;
        let info = r.bytes(info_len)?;
        let padding = r.pad4_lenient(start)?;
        Ok(UserRecord {
            user_id,
            status,
            ignored_05,
            client_port,
            text_color,
            background_color,
            account_color,
            flags,
            ignored_1b,
            icon,
            extra,
            nick,
            info,
            padding,
        })
    }

    pub fn encode(&self, w: &mut Writer) {
        let start = w.pos();
        w.u32(self.user_id);
        w.u8(self.status);
        w.bytes(&self.ignored_05);
        w.u16(self.client_port);
        w.u16(self.icon.len() as u16);
        self.text_color.encode(w);
        self.background_color.encode(w);
        self.account_color.encode(w);
        w.u8(self.flags);
        w.u8(self.extra.len() as u8);
        w.u8(self.nick.len() as u8);
        w.u8(self.info.len() as u8);
        w.u8(self.ignored_1b);
        w.bytes(&self.icon);
        w.bytes(&self.extra);
        w.bytes(&self.nick);
        w.bytes(&self.info);
        if !self.padding.is_empty() {
            w.pad4(start, &self.padding);
        }
    }
}

/// A chat room, as 217 Create Chat, 219 Chat Info, and 220 Set Chat carry it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatRecord {
    pub chat_id: u32,
    pub key: u32,
    pub ignored_08: [u8; 12],
    /// Class bitmaps; the MSB is class 0.
    pub join_classes: u32,
    pub send_classes: u32,
    pub topic_classes: u32,
    pub ignored_20: [u8; 8],
    /// Bit 0 private, bit 1 voice.
    pub flags: u16,
    pub ignored_2e: [u8; 2],
    pub name: Vec<u8>,
    pub description: Vec<u8>,
    /// `host:port` of an IRC gateway, or empty.
    pub irc_server: Vec<u8>,
    pub irc_channel: Vec<u8>,
}

impl ChatRecord {
    pub const FLAG_PRIVATE: u16 = 1;
    pub const FLAG_VOICE: u16 = 2;

    pub fn is_private(&self) -> bool {
        self.flags & Self::FLAG_PRIVATE != 0
    }

    pub fn is_voice(&self) -> bool {
        self.flags & Self::FLAG_VOICE != 0
    }

    pub fn decode(r: &mut Reader) -> Result<Self, DecodeError> {
        let chat_id = r.u32()?;
        let key = r.u32()?;
        let ignored_08: [u8; 12] = r.array()?;
        let join_classes = r.u32()?;
        let send_classes = r.u32()?;
        let topic_classes = r.u32()?;
        let ignored_20: [u8; 8] = r.array()?;
        let flags = r.u16()?;
        let name_len = usize::from(r.u8()?);
        let description_len = usize::from(r.u8()?);
        let irc_server_len = usize::from(r.u8()?);
        let irc_channel_len = usize::from(r.u8()?);
        let ignored_2e: [u8; 2] = r.array()?;
        Ok(ChatRecord {
            chat_id,
            key,
            ignored_08,
            join_classes,
            send_classes,
            topic_classes,
            ignored_20,
            flags,
            ignored_2e,
            name: r.bytes(name_len)?,
            description: r.bytes(description_len)?,
            irc_server: r.bytes(irc_server_len)?,
            irc_channel: r.bytes(irc_channel_len)?,
        })
    }

    pub fn encode(&self, w: &mut Writer) {
        w.u32(self.chat_id);
        w.u32(self.key);
        w.bytes(&self.ignored_08);
        w.u32(self.join_classes);
        w.u32(self.send_classes);
        w.u32(self.topic_classes);
        w.bytes(&self.ignored_20);
        w.u16(self.flags);
        w.u8(self.name.len() as u8);
        w.u8(self.description.len() as u8);
        w.u8(self.irc_server.len() as u8);
        w.u8(self.irc_channel.len() as u8);
        w.bytes(&self.ignored_2e);
        w.bytes(&self.name);
        w.bytes(&self.description);
        w.bytes(&self.irc_server);
        w.bytes(&self.irc_channel);
    }
}
