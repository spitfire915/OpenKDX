#![forbid(unsafe_code)]
//! KDX wire format: handshake, encryption, framing, typed payloads. No I/O.

pub mod wire;

pub use wire::{DecodeError, Reader, Writer};
