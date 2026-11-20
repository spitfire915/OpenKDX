#![forbid(unsafe_code)]
//! Challenge tables: fetch, import, cache, and verify the Haxial stories.
//!
//! The KDX handshake proves knowledge of a challenge table built into the
//! Haxial programs. The tables are Haxial's copyrighted text and are never
//! shipped with OpenKDX; this crate obtains them on the user's machine and
//! caches them under the user's cache directory.

mod cache;
#[cfg(feature = "fetch")]
mod fetch;
mod import;
mod story;

pub use cache::Cache;
#[cfg(feature = "fetch")]
pub use fetch::{Source, fetch, fetch_from, sources};
pub use import::{import, scan_blob};
pub use story::{Story, extract};

/// A ready `ChallengeTable` from the environment or the default cache.
///
/// Order: the story's environment variable (`OPENKDX_KDX_TABLE`,
/// `OPENKDX_HGC_TABLE`; base64, or the literal `default` for the built-in
/// 31-byte line that mocks use — test rigs only), then the cache. Nothing is
/// fetched here; see the `openkdx-table` binary.
pub fn load(story: Story) -> Result<kdx_proto::ChallengeTable, TableError> {
    load_from(story, &Cache::new(Cache::default_dir()))
}

/// [`load`] with an explicit cache.
pub fn load_from(story: Story, cache: &Cache) -> Result<kdx_proto::ChallengeTable, TableError> {
    if std::env::var(story.env_var()).is_ok_and(|v| v.trim() == "default") {
        return Ok(kdx_proto::ChallengeTable::default());
    }
    let bytes = match from_env(story)? {
        Some(bytes) => bytes,
        None => cache.get(story)?.ok_or(TableError::NoSource(story))?,
    };
    kdx_proto::ChallengeTable::from_bytes(story.into(), &bytes).map_err(TableError::Checksum)
}

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
    #[error("no download source is known for {0:?}")]
    NoSource(Story),
    #[cfg(feature = "fetch")]
    #[error("{url}: HTTP {status}")]
    HttpStatus { url: String, status: u16 },
    #[cfg(feature = "fetch")]
    #[error(transparent)]
    Http(#[from] reqwest::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("invalid base64 in environment variable: {0}")]
    Base64(#[from] base64::DecodeError),
    #[error("kdx-proto rejected the table: {0}")]
    Checksum(kdx_proto::challenge::ChallengeError),
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
