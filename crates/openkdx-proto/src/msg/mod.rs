//! Typed message payloads. One struct per message in the reference spec,
//! each implementing [`Payload`]; [`Message`] gathers them.
//!
//! Every payload decodes to exactly the bytes it came from: `ignored_*`,
//! `extra`, and `padding` bytes are kept as fields and written back. Bytes a
//! struct does not account for fail decoding with [`DecodeError::Trailing`].

pub mod common;

use std::fmt;

use crate::message_type::MessageType;
use crate::wire::{DecodeError, Reader, Writer};

/// A message payload with a fixed type number.
pub trait Payload: Sized + fmt::Debug {
    const KIND: MessageType;

    /// Decodes the whole payload; trailing bytes are an error.
    fn decode(bytes: &[u8]) -> Result<Self, DecodeError>;

    fn encode(&self, w: &mut Writer);

    fn to_bytes(&self) -> Vec<u8> {
        let mut w = Writer::new();
        self.encode(&mut w);
        w.into_inner()
    }
}

/// The 16 bytes the server adds to an error [`Status`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusDetail {
    /// ASCII module tag packed into an integer: 0x54524e is `TRN`.
    pub module: u32,
    pub line: u32,
    pub extra: u32,
    pub ignored: u32,
}

/// 1 Status: the reply that carries a result code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    /// 0 for success, else an error code; see [`error_name`].
    pub code: u32,
    /// The request type in an error, or the value in a success.
    pub request_type_or_value: u32,
    /// Only in the server's 24-byte error form.
    pub detail: Option<StatusDetail>,
}

impl Status {
    pub fn is_error(&self) -> bool {
        self.code != 0
    }
}

impl Payload for Status {
    const KIND: MessageType = MessageType::STATUS;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let code = r.u32()?;
        let request_type_or_value = r.u32()?;
        let detail = if r.is_empty() {
            None
        } else {
            Some(StatusDetail {
                module: r.u32()?,
                line: r.u32()?,
                extra: r.u32()?,
                ignored: r.u32()?,
            })
        };
        r.finish()?;
        Ok(Status {
            code,
            request_type_or_value,
            detail,
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.code);
        w.u32(self.request_type_or_value);
        if let Some(d) = &self.detail {
            w.u32(d.module);
            w.u32(d.line);
            w.u32(d.extra);
            w.u32(d.ignored);
        }
    }
}

/// The meaning of an error code, from the server's table of 79 names.
pub fn error_name(code: u32) -> Option<&'static str> {
    Some(match code {
        0x10001 => "unknown error",
        0x10002 => "invalid value",
        0x10003 => "protocol breach",
        0x10004 => "unimplemented",
        0x10005 => "unsupported",
        0x10006 => "timed out",
        0x10007 => "aborted",
        0x10008 => "data version incompatible",
        0x10009 => "data format incompatible",
        0x1000a => "corrupt data",
        0x1000b => "out of range",
        0x1000c => "item does not exist",
        0x1000d => "item already exists",
        0x1000e => "limit cannot be exceeded",
        0x1000f => "locked",
        0x10010 => "data file missing",
        0x10011 => "resource missing",
        0x10012 => "process does not exist",
        0x10013 => "IPC queue full",
        0x10014 => "Access Denied (a missing privilege)",
        0x10015 => "hardware error",
        0x10016 => "hardware busy",
        0x10017 => "unknown view class",
        0x10018 => "view does not exist",
        0x10019 => "invalid object",
        0x1001a => "buffer too small",
        0x1001b => "unknown OS error",
        0x1001c => "not initialized",
        0x1001d => "re-entry not allowed",
        0x1001e => "invalid name",
        0x20064..=0x20066 => "memory errors",
        0x30064..=0x30065 => "file system errors",
        0x30066 => "file, folder, or item missing",
        0x30067 => "file missing",
        0x30068 => "folder missing",
        0x30069 => "alias missing",
        0x3006a => "expected a file",
        0x3006b => "expected a folder",
        0x3006c => "expected an alias",
        0x3006d => "alias target missing",
        0x3006e => "read-only volume",
        0x3006f => "volume closed",
        0x30070 => "no space",
        0x30071 => "volume limit",
        0x30072 => "wrong passphrase",
        0x30073 => "name already exists",
        0x30074 => "end of file",
        0x30075 => "file is open",
        0x30076 => "invalid name",
        0x30077 => "access denied by the file system",
        0x30078 => "cross-volume operation",
        0x30079 => "folder too deep or alias loop",
        0x3007a => "volume missing",
        0x40064..=0x4006f => "network errors",
        0x40070 => "Incorrect login and/or password.",
        0x40071 => "The network protocol is unknown.",
        0x40072 => "the peer's protocol is too old",
        0x40073 => "the peer's protocol is more recent",
        0x40074 => "Received data that was invalid according to the protocol.",
        0x40075 => "A limit in the net protocol was exceeded.",
        0x40076 => "can't send fast enough",
        0x40077 => "Banned for flooding.",
        0x50064..=0x50066 => "sound errors",
        _ => return None,
    })
}

macro_rules! messages {
    ($($variant:ident($ty:path);)*) => {
        /// Every typed payload, or the bytes of one this crate does not model.
        #[derive(Debug, Clone, PartialEq)]
        pub enum Message {
            $($variant($ty),)*
            Unknown { kind: MessageType, bytes: Vec<u8> },
        }

        impl Message {
            /// Decodes `bytes` as the payload of `kind`. An unknown kind gives
            /// [`Message::Unknown`]; a known kind whose bytes do not fit is an error.
            pub fn decode(kind: MessageType, bytes: &[u8]) -> Result<Message, DecodeError> {
                Ok(match kind {
                    $(<$ty as Payload>::KIND => Message::$variant(<$ty as Payload>::decode(bytes)?),)*
                    _ => Message::Unknown { kind, bytes: bytes.to_vec() },
                })
            }

            pub fn kind(&self) -> MessageType {
                match self {
                    $(Message::$variant(_) => <$ty as Payload>::KIND,)*
                    Message::Unknown { kind, .. } => *kind,
                }
            }

            pub fn encode(&self) -> Vec<u8> {
                match self {
                    $(Message::$variant(p) => p.to_bytes(),)*
                    Message::Unknown { bytes, .. } => bytes.clone(),
                }
            }

            /// Whether this crate has a type for `kind`.
            pub fn is_modeled(kind: MessageType) -> bool {
                matches!(kind, $(<$ty as Payload>::KIND)|*)
            }
        }
    };
}

messages! {
    Status(Status);
}
