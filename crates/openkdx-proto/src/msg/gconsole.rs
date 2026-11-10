//! Messages 4000–4201: GConsole, the Haxial remote console the client can
//! display. Names are provisional. See the reference spec's
//! `messages/gconsole.md`.

use crate::message_type::MessageType;
use crate::msg::Payload;
use crate::wire::{DecodeError, Reader, Writer, pad4_len};

/// 4000: opens the window. The payload is at least 24 bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OpenWindow {
    /// 0x40 to 0x1000.
    pub width: u16,
    /// 0x10 to 0x1000.
    pub height: u16,
    /// Fill colour.
    pub color: u32,
    pub rest: [u8; 16],
}

impl Payload for OpenWindow {
    const KIND: MessageType = MessageType::GCONSOLE_OPEN_WINDOW;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let out = OpenWindow {
            width: r.u16()?,
            height: r.u16()?,
            color: r.u32()?,
            rest: r.array()?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.u16(self.width);
        w.u16(self.height);
        w.u32(self.color);
        w.bytes(&self.rest);
    }
}

/// One record of 4001: the top 10 bits of the leading word are the kind, the
/// low 22 bits the body length; the body is padded to a multiple of 4.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DrawRecord {
    pub kind: u16,
    pub body: Vec<u8>,
    pub padding: Vec<u8>,
}

impl DrawRecord {
    pub const KIND_CREATE: u16 = 2;
    pub const KIND_DELETE: u16 = 3;
    pub const KIND_MOVE: u16 = 4;
    pub const KIND_FILL_RECT: u16 = 100;
    pub const KIND_GRADIENT_RECT: u16 = 101;
    pub const KIND_FRAME_RECT: u16 = 102;
    pub const KIND_PIXEL: u16 = 103;
    pub const KIND_LINE: u16 = 104;
    pub const KIND_TEXT_105: u16 = 105;
    pub const KIND_TEXT_106: u16 = 106;
    pub const KIND_POLYGON: u16 = 107;
    pub const KIND_FLOOD_FILL: u16 = 108;
    pub const KIND_COPY: u16 = 109;
    pub const MAX_BODY: usize = 0x3f_ffff;

    /// The object the record draws into: the u16 at the start of every body
    /// except a create's.
    pub fn object_id(&self) -> Option<u16> {
        if self.kind == Self::KIND_CREATE {
            return None;
        }
        self.body.get(..2).map(|b| u16::from_be_bytes([b[0], b[1]]))
    }
}

/// 4001: a stream of draw records.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Draw {
    pub records: Vec<DrawRecord>,
}

impl Payload for Draw {
    const KIND: MessageType = MessageType::GCONSOLE_DRAW;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let mut records = Vec::new();
        while !r.is_empty() {
            let start = r.pos();
            let word = r.u32()?;
            let kind = (word >> 22) as u16;
            let len = (word & 0x3f_ffff) as usize;
            let body = r.bytes(len)?;
            let padding = r.pad4(start)?;
            records.push(DrawRecord {
                kind,
                body,
                padding,
            });
        }
        Ok(Draw { records })
    }

    fn encode(&self, w: &mut Writer) {
        for rec in &self.records {
            let start = w.pos();
            let len = rec.body.len().min(DrawRecord::MAX_BODY);
            w.u32((u32::from(rec.kind) << 22) | len as u32);
            w.bytes(&rec.body[..len]);
            debug_assert_eq!(pad4_len(4 + len), pad4_len(w.pos() - start));
            w.pad4(start, &rec.padding);
        }
    }
}

/// 4200: a key event from the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Key {
    /// The client's clock, ms.
    pub time: u32,
    /// 8 Shift, 2 Alt, 0x21 Ctrl, 4 Caps Lock, 0x30 Windows.
    pub modifiers: u32,
    /// 0 for keys with no character, and on release.
    pub character: u32,
    /// USB HID usage from Client 1.620; Client 1.600 sends its own codes.
    pub usage: u16,
    /// 1 pressed, 2 released.
    pub kind: u8,
    pub value_0f: u8,
}

impl Payload for Key {
    const KIND: MessageType = MessageType::GCONSOLE_KEY;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let out = Key {
            time: r.u32()?,
            modifiers: r.u32()?,
            character: r.u32()?,
            usage: r.u16()?,
            kind: r.u8()?,
            value_0f: r.u8()?,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.time);
        w.u32(self.modifiers);
        w.u32(self.character);
        w.u16(self.usage);
        w.u8(self.kind);
        w.u8(self.value_0f);
    }
}

/// 4201: a mouse button or wheel event from the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Mouse {
    pub time: u32,
    /// 0x00040000 on a right button press.
    pub value_04: u32,
    pub x: u16,
    pub y: u16,
    /// 1 left, 2 right, 3 middle, 4 wheel up, 5 wheel down.
    pub button: u8,
    /// 5 pressed, 6 released, 10 wheel.
    pub kind: u8,
    /// 0x78 up, -0x78 down, 0 for buttons.
    pub wheel: i16,
}

impl Payload for Mouse {
    const KIND: MessageType = MessageType::GCONSOLE_MOUSE;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let out = Mouse {
            time: r.u32()?,
            value_04: r.u32()?,
            x: r.u16()?,
            y: r.u16()?,
            button: r.u8()?,
            kind: r.u8()?,
            wheel: r.u16()? as i16,
        };
        r.finish()?;
        Ok(out)
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.time);
        w.u32(self.value_04);
        w.u16(self.x);
        w.u16(self.y);
        w.u8(self.button);
        w.u8(self.kind);
        w.u16(self.wheel as u16);
    }
}
