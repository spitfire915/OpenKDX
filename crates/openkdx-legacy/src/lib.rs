#![forbid(unsafe_code)]
//! Haxial on-disk formats: Accounts.dat, .stg, .hap, sound lists, news store.
//!
//! Every reader keeps the bytes it does not understand, so that
//! `encode(decode(file)) == file`; see `docs/formats/`.

pub mod accounts;
pub mod bases;
pub mod error;
pub mod news;
pub mod scramble;
pub mod server_stg;
pub mod store;
pub mod tracker_stg;

pub use accounts::AccountsFile;
pub use bases::{BaseName, BaseTag, format_base_name, parse_base_name};
pub use error::{LegacyError, Result};
pub use openkdx_proto::msg::admin::ServerSettings;
pub use store::{LegacyDir, ServerFile};
