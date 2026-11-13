#![forbid(unsafe_code)]
//! Async client library for writing KDX clients.

pub mod error;
pub mod identity;
pub mod table;

pub use error::{ClientError, Result};
pub use identity::Identity;
pub use table::{load_kdx_table, load_kdx_table_from};
pub mod accounts;
pub mod admin;
pub mod chat;
pub mod connection;
pub mod files;
pub mod handshake;
pub mod news;
pub mod path;
pub mod pm;
pub mod session;
pub mod transfer;
pub mod txid;
