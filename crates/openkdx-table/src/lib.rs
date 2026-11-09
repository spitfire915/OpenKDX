#![forbid(unsafe_code)]
//! Challenge tables: fetch, import, cache, and verify the Haxial stories.
//!
//! The KDX handshake proves knowledge of a challenge table built into the
//! Haxial programs. The tables are Haxial's copyrighted text and are never
//! shipped with OpenKDX; this crate obtains them on the user's machine and
//! caches them under the user's cache directory.

mod cache;
mod story;

pub use cache::Cache;
pub use story::{Story, extract};

use base64::Engine;

#[derive(Debug, thiserror::Error)]
pub enum TableError {
    #[error("the story's anchor text was not found in the input")]
    AnchorNotFound,
    #[error(
        "hash mismatch: expected {}, got {}",
        hex::encode(expected),
        hex::encode(actual)
    )]
    HashMismatch {
        expected: [u8; 32],
        actual: [u8; 32],
    },
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("invalid base64 in environment variable: {0}")]
    Base64(#[from] base64::DecodeError),
}

/// Reads a story from its environment variable (`OPENKDX_KDX_TABLE` or
/// `OPENKDX_HGC_TABLE`), base64 encoded. `Ok(None)` when the variable is unset.
pub fn from_env(story: Story) -> Result<Option<Vec<u8>>, TableError> {
    let Ok(value) = std::env::var(story.env_var()) else {
        return Ok(None);
    };
    let bytes = base64::engine::general_purpose::STANDARD.decode(value.trim())?;
    story.verify(&bytes)?;
    Ok(Some(bytes))
}
