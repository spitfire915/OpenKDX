#![forbid(unsafe_code)]
//! KDX wire format: handshake, encryption, framing, typed payloads. No I/O.
//!
//! A connection goes through these steps:
//!
//! 1. Each side sends a [`Hello`](handshake::Hello) under the handshake
//!    [XOR stream](xor::xor_stream).
//! 2. Each side sends a [`Proof`](handshake::Proof) computed from the shared
//!    [`AccountKey`], the peer's nonce, and the [`ChallengeTable`].
//! 3. The session follows: Twofish-256 ECB framed messages.

pub mod account;
pub mod challenge;
pub mod handshake;
pub mod registration;
pub mod session;
pub mod wire;
pub mod xor;

pub use account::AccountKey;
pub use challenge::ChallengeTable;
pub use wire::{DecodeError, Reader, Writer};
