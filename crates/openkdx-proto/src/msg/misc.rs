//! Messages 800–804 (processes), 900–905 (view display), 1000–1003 (voice),
//! and 1100–1107 (folder access). See the reference spec's pages of the same
//! names. The view-display pixel format and the voice codec details are
//! Phase 10 work; their bytes are kept as they are.

use crate::message_type::MessageType;
use crate::msg::common::Time1900;
use crate::msg::{Payload, empty_payload};
use crate::wire::{DecodeError, Reader, Writer};

empty_payload! {
    /// 800: reply is 803, or 1 Status 0x10003 if already open.
    OpenProcessMonitor = OPEN_PROCESS_MONITOR;
    /// 801
    CloseProcessMonitor = CLOSE_PROCESS_MONITOR;
    /// 802: reply is 803.
    GetProcessList = GET_PROCESS_LIST;
    /// 900: on a transfer connection; reply is 902, then 903s.
    OpenViewDisplay = OPEN_VIEW_DISPLAY;
    /// 1000: client to client; reply is 1001.
    StartVoice = START_VOICE;
}

/// One process in 803.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Process {
    pub pid: u32,
    /// 0 on Linux; the high word of a Mac process serial number.
    pub pid_high: u32,
    pub start_time: Time1900,
    pub memory_used: u64,
    pub memory_size: u64,
    /// Seconds.
    pub cpu_time: u32,
    pub background: u32,
    /// 0xff if unknown.
    pub cpu_percent: u8,
    pub memory_percent: u8,
    pub ignored_2a: [u8; 15],
    pub command: Vec<u8>,
    pub string_3b: Vec<u8>,
    pub user: Vec<u8>,
    pub padding: Vec<u8>,
}

/// 803: reply to 800 or 802, or a notification while the monitor is open.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProcessList {
    pub memory_used: u64,
    pub memory_total: u64,
    pub ignored: [u8; 12],
    pub processes: Vec<Process>,
}

impl Payload for ProcessList {
    const KIND: MessageType = MessageType::PROCESS_LIST;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let memory_used = r.u64()?;
        let memory_total = r.u64()?;
        let ignored: [u8; 12] = r.array()?;
        let count = r.u32()? as usize;
        let mut processes = Vec::with_capacity(count.min(4096));
        for _ in 0..count {
            let start = r.pos();
            let pid = r.u32()?;
            let pid_high = r.u32()?;
            let start_time = Time1900::decode(&mut r)?;
            let memory_used = r.u64()?;
            let memory_size = r.u64()?;
            let cpu_time = r.u32()?;
            let background = r.u32()?;
            let cpu_percent = r.u8()?;
            let memory_percent = r.u8()?;
            let ignored_2a: [u8; 15] = r.array()?;
            let user_len = usize::from(r.u8()?);
            let command_len = usize::from(r.u8()?);
            let string_3b_len = usize::from(r.u8()?);
            processes.push(Process {
                pid,
                pid_high,
                start_time,
                memory_used,
                memory_size,
                cpu_time,
                background,
                cpu_percent,
                memory_percent,
                ignored_2a,
                command: r.bytes(command_len)?,
                string_3b: r.bytes(string_3b_len)?,
                user: r.bytes(user_len)?,
                padding: r.pad4(start)?,
            });
        }
        r.finish()?;
        Ok(ProcessList {
            memory_used,
            memory_total,
            ignored,
            processes,
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.u64(self.memory_used);
        w.u64(self.memory_total);
        w.bytes(&self.ignored);
        w.u32(self.processes.len() as u32);
        for p in &self.processes {
            let start = w.pos();
            w.u32(p.pid);
            w.u32(p.pid_high);
            p.start_time.encode(w);
            w.u64(p.memory_used);
            w.u64(p.memory_size);
            w.u32(p.cpu_time);
            w.u32(p.background);
            w.u8(p.cpu_percent);
            w.u8(p.memory_percent);
            w.bytes(&p.ignored_2a);
            w.u8(p.user.len() as u8);
            w.u8(p.command.len() as u8);
            w.u8(p.string_3b.len() as u8);
            w.bytes(&p.command);
            w.bytes(&p.string_3b);
            w.bytes(&p.user);
            w.pad4(start, &p.padding);
        }
    }
}

/// 804: needs Terminate Processes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ExitTerminateProcess {
    /// Bit 0 kill, bit 1 exit, bit 2 restart.
    pub flags: u32,
    pub pid: u32,
    /// 0 on Linux; the high word of a Mac process serial number.
    pub value_08: u32,
}

impl ExitTerminateProcess {
    pub const FLAG_KILL: u32 = 1;
    pub const FLAG_EXIT: u32 = 2;
    pub const FLAG_RESTART: u32 = 4;
}

impl Payload for ExitTerminateProcess {
    const KIND: MessageType = MessageType::TERMINATE_PROCESS;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let out = ExitTerminateProcess {
            flags: r.u32()?,
            pid: r.u32()?,
            value_08: r.u32()?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.flags);
        w.u32(self.pid);
        w.u32(self.value_08);
    }
}

/// 901: nothing sends or handles it; the bytes are kept.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CloseViewDisplay {
    pub ignored: Vec<u8>,
}

impl Payload for CloseViewDisplay {
    const KIND: MessageType = MessageType::CLOSE_VIEW_DISPLAY;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        Ok(CloseViewDisplay {
            ignored: bytes.to_vec(),
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.bytes(&self.ignored);
    }
}

/// 902: reply to 900.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ViewDisplayParams {
    /// Bit 0: the viewer may control the display.
    pub flags: u32,
    pub ignored: [u8; 12],
}

impl Payload for ViewDisplayParams {
    const KIND: MessageType = MessageType::VIEW_DISPLAY_PARAMS;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let out = ViewDisplayParams {
            flags: r.u32()?,
            ignored: r.array()?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.flags);
        w.bytes(&self.ignored);
    }
}

/// 903: a screen update. The pixel stream's format is not pinned down yet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ViewDisplayUpdate {
    pub screen_width: u16,
    pub screen_height: u16,
    pub left: u16,
    pub top: u16,
    /// Not included.
    pub right: u16,
    pub bottom: u16,
    pub pixels: Vec<u8>,
}

impl Payload for ViewDisplayUpdate {
    const KIND: MessageType = MessageType::VIEW_DISPLAY_UPDATE;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        Ok(ViewDisplayUpdate {
            screen_width: r.u16()?,
            screen_height: r.u16()?,
            left: r.u16()?,
            top: r.u16()?,
            right: r.u16()?,
            bottom: r.u16()?,
            pixels: r.rest(),
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.u16(self.screen_width);
        w.u16(self.screen_height);
        w.u16(self.left);
        w.u16(self.top);
        w.u16(self.right);
        w.u16(self.bottom);
        w.bytes(&self.pixels);
    }
}

/// 904
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ViewDisplayMouse {
    pub modifiers: u32,
    pub x: u16,
    pub y: u16,
    pub wheel: u16,
    /// 1 left, 2 right, 3 middle.
    pub button: u8,
    /// 1 down, 2 up, 3 move, 4 wheel.
    pub kind: u8,
}

impl Payload for ViewDisplayMouse {
    const KIND: MessageType = MessageType::VIEW_DISPLAY_MOUSE;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let out = ViewDisplayMouse {
            modifiers: r.u32()?,
            x: r.u16()?,
            y: r.u16()?,
            wheel: r.u16()?,
            button: r.u8()?,
            kind: r.u8()?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.modifiers);
        w.u16(self.x);
        w.u16(self.y);
        w.u16(self.wheel);
        w.u8(self.button);
        w.u8(self.kind);
    }
}

/// 905
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ViewDisplayKeyboard {
    /// Client 1.600: Ctrl 0x21, Alt 2, Shift 8, Caps Lock 4, Windows 0x30.
    pub modifiers: u32,
    pub character: u16,
    pub key_code: u16,
    pub ignored: [u8; 3],
    /// 1 down, 2 up, 3 repeat.
    pub kind: u8,
}

impl Payload for ViewDisplayKeyboard {
    const KIND: MessageType = MessageType::VIEW_DISPLAY_KEYBOARD;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let out = ViewDisplayKeyboard {
            modifiers: r.u32()?,
            character: r.u16()?,
            key_code: r.u16()?,
            ignored: r.array()?,
            kind: r.u8()?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.modifiers);
        w.u16(self.character);
        w.u16(self.key_code);
        w.bytes(&self.ignored);
        w.u8(self.kind);
    }
}

/// 1001: reply to 1000.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StartVoiceReply {
    pub result: u32,
}

impl Payload for StartVoiceReply {
    const KIND: MessageType = MessageType::START_VOICE_REPLY;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let result = r.u32()?;
        r.finish()?;
        Ok(StartVoiceReply { result })
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.result);
    }
}

/// 1002
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceFormat {
    pub ignored: [u8; 12],
    /// Hz.
    pub sample_rate: u32,
    /// `ADPC`.
    pub codec: [u8; 4],
    /// Milliseconds.
    pub buffer: u16,
    pub channels: u8,
    pub bits_per_sample: u8,
}

impl Payload for VoiceFormat {
    const KIND: MessageType = MessageType::SET_VOICE_FORMAT;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let out = VoiceFormat {
            ignored: r.array()?,
            sample_rate: r.u32()?,
            codec: r.array()?,
            buffer: r.u16()?,
            channels: r.u8()?,
            bits_per_sample: r.u8()?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.bytes(&self.ignored);
        w.u32(self.sample_rate);
        w.bytes(&self.codec);
        w.u16(self.buffer);
        w.u8(self.channels);
        w.u8(self.bits_per_sample);
    }
}

/// 1003: IMA ADPCM audio.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct VoiceData {
    pub audio: Vec<u8>,
}

impl Payload for VoiceData {
    const KIND: MessageType = MessageType::VOICE_DATA;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        Ok(VoiceData {
            audio: bytes.to_vec(),
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.bytes(&self.audio);
    }
}

/// One class's permissions in a folder access item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FolderAccessRule {
    /// The MSB is bit 0.
    pub permission_bits: u16,
    pub ignored: u16,
}

/// A folder access item, as 1100, 1104, and 1105 carry it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderAccess {
    pub ignored_00: [u8; 4],
    /// Byte 7: bit 0 built-in, bit 1 Owner Login In Folder Name.
    pub flags: u32,
    /// 0x28 Normal, 0x2b Uploads, 0x2c Dropbox, 0x34 Pictures, 0x35 Audio.
    pub icon: u32,
    /// One rule per class index.
    pub class_rules: [FolderAccessRule; 32],
    /// Probably the owner's rule.
    pub rule: FolderAccessRule,
    pub ignored_90: [u8; 29],
    pub ignored_b0: u8,
    pub ignored_b2: [u8; 6],
    /// The tag, such as `DB`.
    pub name: Vec<u8>,
    pub owner: Vec<u8>,
    pub description: Vec<u8>,
    /// Only in 1105: the item to change.
    pub current_name: Vec<u8>,
    pub padding: Vec<u8>,
}

impl FolderAccess {
    pub const ICON_NORMAL: u32 = 0x28;
    pub const ICON_UPLOADS: u32 = 0x2b;
    pub const ICON_DROPBOX: u32 = 0x2c;
    pub const ICON_PICTURES: u32 = 0x34;
    pub const ICON_AUDIO: u32 = 0x35;

    fn decode_from(r: &mut Reader) -> Result<Self, DecodeError> {
        let start = r.pos();
        let ignored_00: [u8; 4] = r.array()?;
        let flags = r.u32()?;
        let icon = r.u32()?;
        let mut class_rules = [FolderAccessRule::default(); 32];
        for rule in &mut class_rules {
            *rule = FolderAccessRule {
                permission_bits: r.u16()?,
                ignored: r.u16()?,
            };
        }
        let rule = FolderAccessRule {
            permission_bits: r.u16()?,
            ignored: r.u16()?,
        };
        let ignored_90: [u8; 29] = r.array()?;
        let name_len = usize::from(r.u8()?);
        let owner_len = usize::from(r.u8()?);
        let description_len = usize::from(r.u8()?);
        let ignored_b0 = r.u8()?;
        let current_name_len = usize::from(r.u8()?);
        let ignored_b2: [u8; 6] = r.array()?;
        Ok(FolderAccess {
            ignored_00,
            flags,
            icon,
            class_rules,
            rule,
            ignored_90,
            ignored_b0,
            ignored_b2,
            name: r.bytes(name_len)?,
            owner: r.bytes(owner_len)?,
            description: r.bytes(description_len)?,
            current_name: r.bytes(current_name_len)?,
            padding: r.pad4_lenient(start)?,
        })
    }

    fn encode_into(&self, w: &mut Writer) {
        let start = w.pos();
        w.bytes(&self.ignored_00);
        w.u32(self.flags);
        w.u32(self.icon);
        for rule in &self.class_rules {
            w.u16(rule.permission_bits);
            w.u16(rule.ignored);
        }
        w.u16(self.rule.permission_bits);
        w.u16(self.rule.ignored);
        w.bytes(&self.ignored_90);
        w.u8(self.name.len() as u8);
        w.u8(self.owner.len() as u8);
        w.u8(self.description.len() as u8);
        w.u8(self.ignored_b0);
        w.u8(self.current_name.len() as u8);
        w.bytes(&self.ignored_b2);
        w.bytes(&self.name);
        w.bytes(&self.owner);
        w.bytes(&self.description);
        w.bytes(&self.current_name);
        if !self.padding.is_empty() {
            w.pad4(start, &self.padding);
        }
    }
}

macro_rules! folder_access_payload {
    ($(#[$doc:meta] $name:ident = $kind:ident;)*) => {
        $(
            #[$doc]
            #[derive(Debug, Clone, PartialEq, Eq)]
            pub struct $name {
                pub item: FolderAccess,
            }

            impl Payload for $name {
                const KIND: MessageType = MessageType::$kind;

                fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
                    let mut r = Reader::new(bytes);
                    let item = FolderAccess::decode_from(&mut r)?;
                    r.finish()?;
                    Ok($name { item })
                }

                fn encode(&self, w: &mut Writer) {
                    self.item.encode_into(w);
                }
            }
        )*
    };
}

folder_access_payload! {
    /// 1100: reply is 1 Status 0x44c.
    CreateFolderAccess = CREATE_FOLDER_ACCESS;
    /// 1104: reply to 1103.
    FolderAccessInfo = FOLDER_ACCESS_INFO;
    /// 1105: `current_name` selects the item; reply is 1 Status 0x451.
    SetFolderAccess = SET_FOLDER_ACCESS;
}

/// A payload that is an item name with no length byte.
macro_rules! folder_access_name_payload {
    ($(#[$doc:meta] $name:ident = $kind:ident;)*) => {
        $(
            #[$doc]
            #[derive(Debug, Clone, PartialEq, Eq, Default)]
            pub struct $name {
                pub name: Vec<u8>,
            }

            impl Payload for $name {
                const KIND: MessageType = MessageType::$kind;

                fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
                    Ok($name { name: bytes.to_vec() })
                }

                fn encode(&self, w: &mut Writer) {
                    w.bytes(&self.name);
                }
            }
        )*
    };
}

folder_access_name_payload! {
    /// 1101: all users get 1102.
    DeleteFolderAccess = DELETE_FOLDER_ACCESS;
    /// 1102: after a successful 1101.
    FolderAccessDeleted = FOLDER_ACCESS_DELETED;
    /// 1103: reply is 1104.
    GetFolderAccess = GET_FOLDER_ACCESS;
}

/// 1106: empty, or a u32 of flags and then any bytes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GetFolderAccessList {
    pub rest: Vec<u8>,
}

impl GetFolderAccessList {
    /// The first 4 bytes, if present.
    pub fn flags(&self) -> Option<u32> {
        self.rest
            .get(..4)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
}

impl Payload for GetFolderAccessList {
    const KIND: MessageType = MessageType::GET_FOLDER_ACCESS_LIST;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        Ok(GetFolderAccessList {
            rest: bytes.to_vec(),
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.bytes(&self.rest);
    }
}

/// One item in 1107.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FolderAccessItem {
    /// 1 built-in, 2 Owner Login In Folder Name.
    pub flags: u32,
    pub icon: u32,
    pub ignored_08: [u8; 8],
    pub ignored_10: u8,
    pub name: Vec<u8>,
    pub second: Vec<u8>,
    pub description: Vec<u8>,
    pub padding: Vec<u8>,
}

/// 1107: reply to 1106.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FolderAccessList {
    pub items: Vec<FolderAccessItem>,
}

impl Payload for FolderAccessList {
    const KIND: MessageType = MessageType::FOLDER_ACCESS_LIST;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let count = r.u32()? as usize;
        let mut items = Vec::with_capacity(count.min(1024));
        for _ in 0..count {
            let start = r.pos();
            let flags = r.u32()?;
            let icon = r.u32()?;
            let ignored_08: [u8; 8] = r.array()?;
            let ignored_10 = r.u8()?;
            let name_len = usize::from(r.u8()?);
            let second_len = usize::from(r.u8()?);
            let description_len = usize::from(r.u8()?);
            items.push(FolderAccessItem {
                flags,
                icon,
                ignored_08,
                ignored_10,
                name: r.bytes(name_len)?,
                second: r.bytes(second_len)?,
                description: r.bytes(description_len)?,
                padding: r.pad4(start)?,
            });
        }
        r.finish()?;
        Ok(FolderAccessList { items })
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.items.len() as u32);
        for i in &self.items {
            let start = w.pos();
            w.u32(i.flags);
            w.u32(i.icon);
            w.bytes(&i.ignored_08);
            w.u8(i.ignored_10);
            w.u8(i.name.len() as u8);
            w.u8(i.second.len() as u8);
            w.u8(i.description.len() as u8);
            w.bytes(&i.name);
            w.bytes(&i.second);
            w.bytes(&i.description);
            w.pad4(start, &i.padding);
        }
    }
}
