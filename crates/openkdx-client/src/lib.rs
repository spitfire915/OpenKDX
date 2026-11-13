#![forbid(unsafe_code)]
//! Async client library for writing KDX clients.

pub mod error;
pub mod identity;
pub mod table;

pub use error::{ClientError, Result};
pub use identity::Identity;
pub use table::{load_kdx_table, load_kdx_table_from};
pub mod handshake;
pub mod txid;
