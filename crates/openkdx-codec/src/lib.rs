#![forbid(unsafe_code)]
//! tokio-util codecs for KDX handshake and session framing.
//!
//! [`HandshakeCodec`] frames the two hellos and two proofs that start a
//! connection; [`SessionCodec`] frames the Twofish-encrypted messages that
//! follow, including the streaming transfer data of 414 and 417.

mod handshake;
mod session;

pub use handshake::{HandshakeCodec, HandshakeFrame};
pub use session::{Event, Item, SessionCodec};

use openkdx_proto::handshake::HandshakeError;
use openkdx_proto::session::SessionError;

#[derive(Debug, thiserror::Error)]
pub enum CodecError {
    #[error(transparent)]
    Session(#[from] SessionError),
    #[error(transparent)]
    Handshake(#[from] HandshakeError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("transfer data overrun: {sent} bytes sent of {declared} declared")]
    DataOverrun { sent: u64, declared: u64 },
    #[error("{0}")]
    Protocol(&'static str),
}
