//! Messages 502–551: account classes, accounts, and passwords, with the
//! privilege bits. See the reference spec's `messages/accounts.md` and
//! `privileges.md`.

use std::ops::{BitAnd, BitOr, Not};

use crate::message_type::MessageType;
use crate::msg::common::{Color, Time1900};
use crate::msg::{Payload, empty_payload};
use crate::wire::{DecodeError, Reader, Writer};

macro_rules! privileges {
    ($($bit:literal $variant:ident $name:literal;)*) => {
        /// A named privilege bit. The number is the bit's position in the
        /// 128-bit set: bit 0 is the high bit of the first byte.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[repr(u8)]
        pub enum Privilege {
            $($variant = $bit,)*
        }

        impl Privilege {
            pub const ALL: &'static [Privilege] = &[$(Privilege::$variant,)*];

            pub fn bit(self) -> u8 {
                self as u8
            }

            /// The label in the client's account editor.
            pub fn name(self) -> &'static str {
                match self {
                    $(Privilege::$variant => $name,)*
                }
            }

            pub fn from_bit(bit: u8) -> Option<Privilege> {
                match bit {
                    $($bit => Some(Privilege::$variant),)*
                    _ => None,
                }
            }
        }
    };
}

privileges! {
    0 Override "Override";
    2 Unnamed2 "(unnamed bit 2)";
    3 ColorOverridesClass "Color Overrides Class";
    4 CanLogin "Can Login";
    10 Download "Download";
    11 Upload "Upload";
    13 EmptyTrash "Empty Trash";
    15 CreateAliases "Create Aliases";
    16 Move "Move";
    17 Rename "Rename";
    18 Delete "Delete";
    20 SetAttributes "Set File/Folder Attributes";
    22 SeeAllVolumes "See All Volumes";
    23 HasCustomBaseFolder "Has Custom Base Folder";
    24 SeeAllBases "See All Bases";
    25 AllowSimultaneousDownloads "Allow Simultaneous Downloads";
    26 IgnoreSpeedLimits "Ignore Speed Limits";
    27 ViewFolderAccess "View Folder Access";
    28 ModifyFolderAccess "Modify Folder Access";
    30 JoinChat "Join Chat";
    31 SendChat "Send Chat";
    32 SendMessages "Send Messages";
    33 Broadcast "Broadcast";
    34 CreatePrivateChat "Create Private Chat";
    35 SetChatTopic "Set Chat Topic";
    36 JoinChatOnConnect "Join Chat On Connect";
    37 CreateModifyChats "Create/Modify Chats";
    40 GetUserInfo "Get User Info";
    41 DisconnectUsers "Disconnect Users";
    42 CannotBeDisconnected "Cannot Be Disconnected";
    43 BanUsers "Ban Users";
    50 ViewAccounts "View Accounts";
    51 CreateAccounts "Create Accounts";
    52 ModifyAccounts "Modify Accounts";
    53 ChangeOwnPassword "Change Own Password";
    54 ProtectedAccount "Protected Account";
    60 UseAnyName "Use Any Name";
    61 ModifyServerSettings "Modify Server Settings";
    62 DontShowGreeting "Don't Show Greeting";
    63 ViewServerHistory "View Server History";
    64 ShutdownServer "Shutdown Server";
    65 DisableEncryption "Disable Encryption";
    66 ViewDisplay "View Display";
    67 ControlDisplay "Control Display";
    70 ViewNews "View News";
    71 PostNews "Post News";
    72 DeleteNewsMessages "Delete News Messages";
    73 CreateModifyNewsgroups "Create/Modify Newsgroups";
    74 SeeLoginsInNews "See Logins In News";
    80 ViewProcesses "View Processes";
    81 TerminateProcesses "Terminate Processes";
    82 LaunchPrograms "Launch Programs";
    83 Unnamed83 "(unnamed bit 83)";
    84 Unnamed84 "(unnamed bit 84)";
    85 Unnamed85 "(unnamed bit 85)";
}

/// A 128-bit privilege set, big-endian: bit n is byte n/8, mask 0x80 >> (n % 8).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Privileges([u8; 16]);

impl Privileges {
    pub const NONE: Privileges = Privileges([0; 16]);
    pub const LEN: usize = 16;

    pub fn from_bytes(bytes: [u8; 16]) -> Self {
        Privileges(bytes)
    }

    pub fn from_bits(bits: &[Privilege]) -> Self {
        let mut p = Self::NONE;
        for b in bits {
            p.set(*b, true);
        }
        p
    }

    pub fn bytes(&self) -> &[u8; 16] {
        &self.0
    }

    pub fn has_bit(&self, bit: u8) -> bool {
        self.0[usize::from(bit / 8)] & (0x80 >> (bit % 8)) != 0
    }

    pub fn has(&self, p: Privilege) -> bool {
        self.has_bit(p.bit())
    }

    pub fn set_bit(&mut self, bit: u8, on: bool) {
        let mask = 0x80 >> (bit % 8);
        let byte = &mut self.0[usize::from(bit / 8)];
        if on {
            *byte |= mask;
        } else {
            *byte &= !mask;
        }
    }

    pub fn set(&mut self, p: Privilege, on: bool) {
        self.set_bit(p.bit(), on);
    }

    /// The named bits that are set, in bit order.
    pub fn set_bits(&self) -> Vec<Privilege> {
        Privilege::ALL
            .iter()
            .copied()
            .filter(|p| self.has(*p))
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.0 == [0; 16]
    }

    fn decode(r: &mut Reader) -> Result<Self, DecodeError> {
        Ok(Privileges(r.array()?))
    }

    fn encode(&self, w: &mut Writer) {
        w.bytes(&self.0);
    }
}

impl std::fmt::Debug for Privileges {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Privileges(")?;
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        write!(f, ")")
    }
}

impl BitAnd for Privileges {
    type Output = Privileges;
    fn bitand(self, rhs: Self) -> Self {
        Privileges(std::array::from_fn(|i| self.0[i] & rhs.0[i]))
    }
}

impl BitOr for Privileges {
    type Output = Privileges;
    fn bitor(self, rhs: Self) -> Self {
        Privileges(std::array::from_fn(|i| self.0[i] | rhs.0[i]))
    }
}

impl Not for Privileges {
    type Output = Privileges;
    fn not(self) -> Self {
        Privileges(self.0.map(|b| !b))
    }
}

empty_payload! {
    /// 505: reply is 506.
    GetAccountClassList = GET_ACCOUNT_CLASS_LIST;
    /// 512: reply is 513.
    GetAccountList = GET_ACCOUNT_LIST;
    /// 551: reply to 550.
    ChangeOwnPasswordDone = CHANGE_OWN_PASSWORD_DONE;
}

/// 502: reply is 503.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct GetAccountClass {
    pub class_index: u8,
}

impl Payload for GetAccountClass {
    const KIND: MessageType = MessageType::GET_ACCOUNT_CLASS;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let class_index = r.u8()?;
        r.finish()?;
        Ok(GetAccountClass { class_index })
    }

    fn encode(&self, w: &mut Writer) {
        w.u8(self.class_index);
    }
}

/// 503: reply to 502. Also the body of 504.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AccountClassInfo {
    pub privileges: Privileges,
    pub time: Time1900,
    pub ignored_18: [u8; 13],
    /// Commands per 2 minutes: 0x50 Relaxed, 0x3c Average, 0x28 Strict.
    pub flood_protection: u8,
    /// KiB/s per download; 0 for no limit.
    pub speed_limit: u16,
    pub ignored_28: [u8; 8],
    pub class_index: u8,
    pub color: Color,
    pub ignored_37: u8,
    pub name: Vec<u8>,
    pub second_string: Vec<u8>,
    /// The login that last edited the class; the server sets it.
    pub editor_login: Vec<u8>,
    pub padding: Vec<u8>,
}

impl AccountClassInfo {
    pub const FLOOD_RELAXED: u8 = 0x50;
    pub const FLOOD_AVERAGE: u8 = 0x3c;
    pub const FLOOD_STRICT: u8 = 0x28;

    fn decode_from(r: &mut Reader) -> Result<Self, DecodeError> {
        let start = r.pos();
        let privileges = Privileges::decode(r)?;
        let time = Time1900::decode(r)?;
        let ignored_18: [u8; 13] = r.array()?;
        let flood_protection = r.u8()?;
        let speed_limit = r.u16()?;
        let ignored_28: [u8; 8] = r.array()?;
        let class_index = r.u8()?;
        let color = Color::decode(r)?;
        let name_len = usize::from(r.u8()?);
        let second_string_len = usize::from(r.u8()?);
        let editor_login_len = usize::from(r.u8()?);
        let ignored_37 = r.u8()?;
        Ok(AccountClassInfo {
            privileges,
            time,
            ignored_18,
            flood_protection,
            speed_limit,
            ignored_28,
            class_index,
            color,
            ignored_37,
            name: r.bytes(name_len)?,
            second_string: r.bytes(second_string_len)?,
            editor_login: r.bytes(editor_login_len)?,
            padding: r.pad4_lenient(start)?,
        })
    }

    fn encode_into(&self, w: &mut Writer) {
        let start = w.pos();
        self.privileges.encode(w);
        self.time.encode(w);
        w.bytes(&self.ignored_18);
        w.u8(self.flood_protection);
        w.u16(self.speed_limit);
        w.bytes(&self.ignored_28);
        w.u8(self.class_index);
        self.color.encode(w);
        w.u8(self.name.len() as u8);
        w.u8(self.second_string.len() as u8);
        w.u8(self.editor_login.len() as u8);
        w.u8(self.ignored_37);
        w.bytes(&self.name);
        w.bytes(&self.second_string);
        w.bytes(&self.editor_login);
        if !self.padding.is_empty() {
            w.pad4(start, &self.padding);
        }
    }
}

impl Payload for AccountClassInfo {
    const KIND: MessageType = MessageType::ACCOUNT_CLASS_INFO;

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

/// 504: a 503 record; reply is 1 Status 0x1f8.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SetAccountClass {
    pub class: AccountClassInfo,
}

impl Payload for SetAccountClass {
    const KIND: MessageType = MessageType::SET_ACCOUNT_CLASS;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let class = AccountClassInfo::decode_from(&mut r)?;
        r.finish()?;
        Ok(SetAccountClass { class })
    }

    fn encode(&self, w: &mut Writer) {
        self.class.encode_into(w);
    }
}

/// One class in 506.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AccountClassListEntry {
    pub privileges: Privileges,
    pub time: Time1900,
    pub ignored_18: [u8; 13],
    pub flood_protection: u8,
    pub speed_limit: u16,
    /// 0 to 11, in list order.
    pub class_index: u8,
    pub color: Color,
    pub ignored_2d: [u8; 2],
    pub ignored_2f: u8,
    pub name: Vec<u8>,
    pub padding: Vec<u8>,
}

/// 506: reply to 505.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AccountClassList {
    pub classes: Vec<AccountClassListEntry>,
}

impl Payload for AccountClassList {
    const KIND: MessageType = MessageType::ACCOUNT_CLASS_LIST;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let count = r.u32()? as usize;
        let mut classes = Vec::with_capacity(count.min(12));
        for _ in 0..count {
            let start = r.pos();
            let privileges = Privileges::decode(&mut r)?;
            let time = Time1900::decode(&mut r)?;
            let ignored_18: [u8; 13] = r.array()?;
            let flood_protection = r.u8()?;
            let speed_limit = r.u16()?;
            let class_index = r.u8()?;
            let color = Color::decode(&mut r)?;
            let name_len = usize::from(r.u8()?);
            let ignored_2d: [u8; 2] = r.array()?;
            let ignored_2f = r.u8()?;
            classes.push(AccountClassListEntry {
                privileges,
                time,
                ignored_18,
                flood_protection,
                speed_limit,
                class_index,
                color,
                ignored_2d,
                ignored_2f,
                name: r.bytes(name_len)?,
                padding: r.pad4(start)?,
            });
        }
        r.finish()?;
        Ok(AccountClassList { classes })
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.classes.len() as u32);
        for c in &self.classes {
            let start = w.pos();
            c.privileges.encode(w);
            c.time.encode(w);
            w.bytes(&c.ignored_18);
            w.u8(c.flood_protection);
            w.u16(c.speed_limit);
            w.u8(c.class_index);
            c.color.encode(w);
            w.u8(c.name.len() as u8);
            w.bytes(&c.ignored_2d);
            w.u8(c.ignored_2f);
            w.bytes(&c.name);
            w.pad4(start, &c.padding);
        }
    }
}

/// An account, as 507, 510, and 511 carry it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Account {
    /// The values of the bits set in `privilege_mask`.
    pub privileges: Privileges,
    /// The bits the account sets itself; the rest come from its class.
    pub privilege_mask: Privileges,
    /// 507: the new key. 511: the new key, or all zeros to keep the current
    /// one. 510: byte 0 is the has-password flag.
    pub key: [u8; 32],
    pub created: Time1900,
    pub modified: Time1900,
    /// 0 if never.
    pub last_login: Time1900,
    pub ignored_58: [u8; 30],
    pub class_index: u8,
    /// Used instead of the class's colour with Color Overrides Class.
    pub color: Color,
    pub ignored_7e: [u8; 2],
    pub login: Vec<u8>,
    pub name: Vec<u8>,
    pub third: Vec<u8>,
    /// The login that edited the account.
    pub editor: Vec<u8>,
    pub padding: Vec<u8>,
}

impl Account {
    /// In 510 Account Info, whether the account has a password.
    pub fn has_password(&self) -> bool {
        self.key[0] != 0
    }

    fn decode_from(r: &mut Reader) -> Result<Self, DecodeError> {
        let start = r.pos();
        let privileges = Privileges::decode(r)?;
        let privilege_mask = Privileges::decode(r)?;
        let key: [u8; 32] = r.array()?;
        let created = Time1900::decode(r)?;
        let modified = Time1900::decode(r)?;
        let last_login = Time1900::decode(r)?;
        let ignored_58: [u8; 30] = r.array()?;
        let class_index = r.u8()?;
        let color = Color::decode(r)?;
        let login_len = usize::from(r.u8()?);
        let name_len = usize::from(r.u8()?);
        let third_len = usize::from(r.u8()?);
        let editor_len = usize::from(r.u8()?);
        let ignored_7e: [u8; 2] = r.array()?;
        Ok(Account {
            privileges,
            privilege_mask,
            key,
            created,
            modified,
            last_login,
            ignored_58,
            class_index,
            color,
            ignored_7e,
            login: r.bytes(login_len)?,
            name: r.bytes(name_len)?,
            third: r.bytes(third_len)?,
            editor: r.bytes(editor_len)?,
            padding: r.pad4_lenient(start)?,
        })
    }

    fn encode_into(&self, w: &mut Writer) {
        let start = w.pos();
        self.privileges.encode(w);
        self.privilege_mask.encode(w);
        w.bytes(&self.key);
        self.created.encode(w);
        self.modified.encode(w);
        self.last_login.encode(w);
        w.bytes(&self.ignored_58);
        w.u8(self.class_index);
        self.color.encode(w);
        w.u8(self.login.len() as u8);
        w.u8(self.name.len() as u8);
        w.u8(self.third.len() as u8);
        w.u8(self.editor.len() as u8);
        w.bytes(&self.ignored_7e);
        w.bytes(&self.login);
        w.bytes(&self.name);
        w.bytes(&self.third);
        w.bytes(&self.editor);
        if !self.padding.is_empty() {
            w.pad4(start, &self.padding);
        }
    }
}

macro_rules! account_payload {
    ($(#[$doc:meta] $name:ident = $kind:ident;)*) => {
        $(
            #[$doc]
            #[derive(Debug, Clone, PartialEq, Eq, Default)]
            pub struct $name {
                pub account: Account,
            }

            impl Payload for $name {
                const KIND: MessageType = MessageType::$kind;

                fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
                    let mut r = Reader::new(bytes);
                    let account = Account::decode_from(&mut r)?;
                    r.finish()?;
                    Ok($name { account })
                }

                fn encode(&self, w: &mut Writer) {
                    self.account.encode_into(w);
                }
            }
        )*
    };
}

account_payload! {
    /// 507: reply is 1 Status 0x1fb.
    CreateAccount = CREATE_ACCOUNT;
    /// 510: reply to 509.
    AccountInfo = ACCOUNT_INFO;
    /// 511: reply is 1 Status 0x1ff.
    SetAccount = SET_ACCOUNT;
}

/// A payload that is a login name with no length byte.
macro_rules! login_payload {
    ($(#[$doc:meta] $name:ident = $kind:ident;)*) => {
        $(
            #[$doc]
            #[derive(Debug, Clone, PartialEq, Eq, Default)]
            pub struct $name {
                pub login: Vec<u8>,
            }

            impl Payload for $name {
                const KIND: MessageType = MessageType::$kind;

                fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
                    Ok($name { login: bytes.to_vec() })
                }

                fn encode(&self, w: &mut Writer) {
                    w.bytes(&self.login);
                }
            }
        )*
    };
}

login_payload! {
    /// 508: all users get 515.
    DeleteAccount = DELETE_ACCOUNT;
    /// 509: reply is 510.
    GetAccount = GET_ACCOUNT;
    /// 515: after a successful 508.
    AccountDeleted = ACCOUNT_DELETED;
}

/// One account in 513.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AccountListEntry {
    pub created: Time1900,
    pub last_login: Time1900,
    pub ignored_10: [u8; 11],
    pub ignored_1b: u8,
    pub ignored_1c: [u8; 4],
    pub class_index: u8,
    pub ignored_21: u8,
    pub ignored_22: [u8; 2],
    pub ignored_26: [u8; 2],
    pub login: Vec<u8>,
    pub name: Vec<u8>,
    pub padding: Vec<u8>,
}

/// 513: reply to 512.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AccountList {
    pub accounts: Vec<AccountListEntry>,
}

impl Payload for AccountList {
    const KIND: MessageType = MessageType::ACCOUNT_LIST;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let count = r.u32()? as usize;
        let mut accounts = Vec::with_capacity(count.min(4096));
        for _ in 0..count {
            let start = r.pos();
            let created = Time1900::decode(&mut r)?;
            let last_login = Time1900::decode(&mut r)?;
            let ignored_10: [u8; 11] = r.array()?;
            let ignored_1b = r.u8()?;
            let ignored_1c: [u8; 4] = r.array()?;
            let class_index = r.u8()?;
            let ignored_21 = r.u8()?;
            let ignored_22: [u8; 2] = r.array()?;
            let login_len = usize::from(r.u8()?);
            let name_len = usize::from(r.u8()?);
            let ignored_26: [u8; 2] = r.array()?;
            accounts.push(AccountListEntry {
                created,
                last_login,
                ignored_10,
                ignored_1b,
                ignored_1c,
                class_index,
                ignored_21,
                ignored_22,
                ignored_26,
                login: r.bytes(login_len)?,
                name: r.bytes(name_len)?,
                padding: r.pad4(start)?,
            });
        }
        r.finish()?;
        Ok(AccountList { accounts })
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.accounts.len() as u32);
        for a in &self.accounts {
            let start = w.pos();
            a.created.encode(w);
            a.last_login.encode(w);
            w.bytes(&a.ignored_10);
            w.u8(a.ignored_1b);
            w.bytes(&a.ignored_1c);
            w.u8(a.class_index);
            w.u8(a.ignored_21);
            w.bytes(&a.ignored_22);
            w.u8(a.login.len() as u8);
            w.u8(a.name.len() as u8);
            w.bytes(&a.ignored_26);
            w.bytes(&a.login);
            w.bytes(&a.name);
            w.pad4(start, &a.padding);
        }
    }
}

/// 514: never sent by Server 1.620; the bytes are kept.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AccountChanged {
    pub ignored: Vec<u8>,
}

impl Payload for AccountChanged {
    const KIND: MessageType = MessageType::ACCOUNT_CHANGED;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        Ok(AccountChanged {
            ignored: bytes.to_vec(),
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.bytes(&self.ignored);
    }
}

/// 550: reply is 551.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeOwnPassword {
    /// The `AccountKey` of the new password.
    pub account_key: [u8; 32],
}

impl Payload for ChangeOwnPassword {
    const KIND: MessageType = MessageType::CHANGE_OWN_PASSWORD;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let account_key: [u8; 32] = r.array()?;
        r.finish()?;
        Ok(ChangeOwnPassword { account_key })
    }

    fn encode(&self, w: &mut Writer) {
        w.bytes(&self.account_key);
    }
}
