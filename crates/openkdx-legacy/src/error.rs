use std::path::PathBuf;

use openkdx_proto::DecodeError;

#[derive(Debug, thiserror::Error)]
pub enum LegacyError {
    #[error("{path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("decode: {0}")]
    Decode(#[from] DecodeError),
    #[error("bad magic: expected {expected:02x?}, found {found:02x?}")]
    BadMagic { expected: [u8; 8], found: [u8; 8] },
    #[error("{file}: unsupported version {version}")]
    UnsupportedVersion { file: &'static str, version: u16 },
    #[error("{0}")]
    Invalid(&'static str),
    #[error("file is not scrambled in the expected way")]
    NotScrambled,
}

pub type Result<T> = std::result::Result<T, LegacyError>;

impl LegacyError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        LegacyError::Io {
            path: path.into(),
            source,
        }
    }
}
