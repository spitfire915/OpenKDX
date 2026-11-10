//! Messages 100–118: identity, the user list, server information, the
//! greeting, ping, SKPK. See the reference spec's `messages/session.md`.

use crate::message_type::MessageType;
use crate::msg::common::{Time1900, UserRecord};
use crate::msg::{Payload, empty_payload, user_record_payload};
use crate::wire::{DecodeError, Reader, Writer};

user_record_payload! {
    /// 100: the client's identity; `user_id` is 0. Sent first after the handshake.
    SetIdentity = SET_IDENTITY;
    /// 102: a user changed nick, icon, or info.
    UserChanged = USER_CHANGED;
    /// 103: a user logged in.
    UserJoined = USER_JOINED;
}

/// 101: every user logged in, each record padded to a multiple of 4.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UserList {
    pub users: Vec<UserRecord>,
}

impl Payload for UserList {
    const KIND: MessageType = MessageType::USER_LIST;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let mut users = Vec::new();
        while !r.is_empty() {
            users.push(UserRecord::decode(&mut r)?);
        }
        Ok(UserList { users })
    }

    fn encode(&self, w: &mut Writer) {
        for u in &self.users {
            u.encode(w);
        }
    }
}

/// 104: a user left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserDeparted {
    pub user_id: u32,
    /// 7 shows a notice in shared chats; 9 is quiet.
    pub kind: u8,
    pub ignored: [u8; 3],
}

impl UserDeparted {
    pub const KIND_NOTICE: u8 = 7;
    pub const KIND_QUIET: u8 = 9;
}

impl Payload for UserDeparted {
    const KIND: MessageType = MessageType::USER_DEPARTED;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let out = UserDeparted {
            user_id: r.u32()?,
            kind: r.u8()?,
            ignored: r.array()?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.user_id);
        w.u8(self.kind);
        w.bytes(&self.ignored);
    }
}

/// 105: sent after the first 100 Set Identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServerInfo {
    /// The key an attached transfer connection logs in with.
    pub session_key: [u8; 32],
    /// The optional File Transfer port, or 0.
    pub file_transfer_port: u32,
    pub user_id: u32,
    /// 0x0654 = 1620.
    pub version: u16,
    pub can_modify_settings: u16,
    pub ignored_2c: [u8; 26],
    /// 2 from Server 1.620; any other value makes the client compare versions.
    pub value_46: u16,
    pub ignored_4b: u8,
    pub name: Vec<u8>,
    pub description: Vec<u8>,
    pub address: Vec<u8>,
}

impl Payload for ServerInfo {
    const KIND: MessageType = MessageType::SERVER_INFO;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let session_key: [u8; 32] = r.array()?;
        let file_transfer_port = r.u32()?;
        let user_id = r.u32()?;
        let version = r.u16()?;
        let can_modify_settings = r.u16()?;
        let ignored_2c: [u8; 26] = r.array()?;
        let value_46 = r.u16()?;
        let name_len = usize::from(r.u8()?);
        let description_len = usize::from(r.u8()?);
        let address_len = usize::from(r.u8()?);
        let ignored_4b = r.u8()?;
        let out = ServerInfo {
            session_key,
            file_transfer_port,
            user_id,
            version,
            can_modify_settings,
            ignored_2c,
            value_46,
            ignored_4b,
            name: r.bytes(name_len)?,
            description: r.bytes(description_len)?,
            address: r.bytes(address_len)?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.bytes(&self.session_key);
        w.u32(self.file_transfer_port);
        w.u32(self.user_id);
        w.u16(self.version);
        w.u16(self.can_modify_settings);
        w.bytes(&self.ignored_2c);
        w.u16(self.value_46);
        w.u8(self.name.len() as u8);
        w.u8(self.description.len() as u8);
        w.u8(self.address.len() as u8);
        w.u8(self.ignored_4b);
        w.bytes(&self.name);
        w.bytes(&self.description);
        w.bytes(&self.address);
    }
}

/// 106: the server's icon bitmap, as in the settings blob.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ServerIcon {
    pub icon: Vec<u8>,
}

impl Payload for ServerIcon {
    const KIND: MessageType = MessageType::SERVER_ICON;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        Ok(ServerIcon {
            icon: bytes.to_vec(),
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.bytes(&self.icon);
    }
}

/// 107: the greeting text, or `(No greeting.)` / `(Greeting disabled.)`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ServerGreeting {
    pub ignored: [u8; 4],
    pub text: Vec<u8>,
}

impl Payload for ServerGreeting {
    const KIND: MessageType = MessageType::SERVER_GREETING;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        Ok(ServerGreeting {
            ignored: r.array()?,
            text: r.rest(),
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.bytes(&self.ignored);
        w.bytes(&self.text);
    }
}

empty_payload! {
    /// 108: does not reset the idle timer.
    Ping = PING;
    /// 109: reply to 108.
    Pong = PONG;
    /// 112: asks for the greeting again; the server answers with a new 107.
    RequestGreeting = REQUEST_GREETING;
    /// 113: reply is 114.
    GetExtraServerInfo = GET_EXTRA_SERVER_INFO;
    /// 115: reply is 116.
    GetMyAddress = GET_MY_ADDRESS;
}

/// The 4-byte mini icon value: 1 after `/away`, 0 after `/back`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MiniUserIcon {
    pub status: u8,
    pub ignored: [u8; 3],
}

impl MiniUserIcon {
    fn decode(r: &mut Reader) -> Result<Self, DecodeError> {
        Ok(MiniUserIcon {
            status: r.u8()?,
            ignored: r.array()?,
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.u8(self.status);
        w.bytes(&self.ignored);
    }
}

/// 110
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SetMiniUserIcon {
    pub icon: MiniUserIcon,
}

impl Payload for SetMiniUserIcon {
    const KIND: MessageType = MessageType::SET_MINI_USER_ICON;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let icon = MiniUserIcon::decode(&mut r)?;
        r.finish()?;
        Ok(SetMiniUserIcon { icon })
    }

    fn encode(&self, w: &mut Writer) {
        self.icon.encode(w);
    }
}

/// 111
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MiniUserIconChanged {
    pub user_id: u32,
    pub icon: MiniUserIcon,
}

impl Payload for MiniUserIconChanged {
    const KIND: MessageType = MessageType::MINI_USER_ICON_CHANGED;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let out = MiniUserIconChanged {
            user_id: r.u32()?,
            icon: MiniUserIcon::decode(&mut r)?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.user_id);
        self.icon.encode(w);
    }
}

/// 114: reply to 113.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtraServerInfo {
    pub ignored_00: [u8; 16],
    /// UTC.
    pub start_time: Time1900,
    /// The server's local time.
    pub now: Time1900,
    /// Seconds.
    pub uptime: u32,
    /// 0x1000 Linux and Mac OS X, 0x10 Mac OS, 0x500 Windows NT, 0x300 Windows 9x.
    pub os_kind: u32,
    pub os_version: u32,
    pub ignored_2d: [u8; 3],
    pub os: Vec<u8>,
}

impl Payload for ExtraServerInfo {
    const KIND: MessageType = MessageType::EXTRA_SERVER_INFO;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let ignored_00: [u8; 16] = r.array()?;
        let start_time = Time1900::decode(&mut r)?;
        let now = Time1900::decode(&mut r)?;
        let uptime = r.u32()?;
        let os_kind = r.u32()?;
        let os_version = r.u32()?;
        let os_len = usize::from(r.u8()?);
        let ignored_2d: [u8; 3] = r.array()?;
        let os = r.bytes(os_len)?;
        r.finish()?;
        Ok(ExtraServerInfo {
            ignored_00,
            start_time,
            now,
            uptime,
            os_kind,
            os_version,
            ignored_2d,
            os,
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.bytes(&self.ignored_00);
        self.start_time.encode(w);
        self.now.encode(w);
        w.u32(self.uptime);
        w.u32(self.os_kind);
        w.u32(self.os_version);
        w.u8(self.os.len() as u8);
        w.bytes(&self.ignored_2d);
        w.bytes(&self.os);
    }
}

/// 116: the client's address as the server sees it, `a.b.c.d:port`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct YourAddress {
    pub address: Vec<u8>,
}

impl Payload for YourAddress {
    const KIND: MessageType = MessageType::YOUR_ADDRESS;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let len = usize::from(r.u8()?);
        let address = r.bytes(len)?;
        r.finish()?;
        Ok(YourAddress { address })
    }

    fn encode(&self, w: &mut Writer) {
        w.u8(self.address.len() as u8);
        w.bytes(&self.address);
    }
}

/// 117: a client's key and listener for a DCC peer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skpk {
    /// The sender's key for secondary connections to it.
    pub key: [u8; 32],
    pub dcc_port: u32,
    pub connection_id: u32,
    /// 0x0654 = 1620.
    pub client_version: u16,
    pub ignored: [u8; 6],
}

impl Payload for Skpk {
    const KIND: MessageType = MessageType::SKPK;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let out = Skpk {
            key: r.array()?,
            dcc_port: r.u32()?,
            connection_id: r.u32()?,
            client_version: r.u16()?,
            ignored: r.array()?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.bytes(&self.key);
        w.u32(self.dcc_port);
        w.u32(self.connection_id);
        w.u16(self.client_version);
        w.bytes(&self.ignored);
    }
}

/// 118: the length of the client's purchase certificate, or 0. OpenKDX
/// presents itself as a registered Client 1.600, so it never sends 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LicenseReport {
    pub value: u32,
}

impl Payload for LicenseReport {
    const KIND: MessageType = MessageType::LICENSE_REPORT;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let value = r.u32()?;
        r.finish()?;
        Ok(LicenseReport { value })
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.value);
    }
}
