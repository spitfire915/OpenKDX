//! On-disk cache for the challenge tables.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::{Story, TableError};

/// A directory holding `kdx.bin` and `hgc.bin`.
#[derive(Debug, Clone)]
pub struct Cache {
    dir: PathBuf,
}

impl Cache {
    /// The per-user cache directory: `<cache_dir>/openkdx/table`.
    pub fn default_dir() -> PathBuf {
        directories::ProjectDirs::from("org", "openkdx", "openkdx")
            .map(|d| d.cache_dir().join("table"))
            .unwrap_or_else(|| PathBuf::from(".openkdx-cache/table"))
    }

    pub fn new(dir: PathBuf) -> Self {
        Cache { dir }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn path_for(&self, story: Story) -> PathBuf {
        self.dir.join(story.file_name())
    }

    /// Returns the cached story if present and intact; `None` if absent or corrupt.
    pub fn get(&self, story: Story) -> Result<Option<Vec<u8>>, TableError> {
        match fs::read(self.path_for(story)) {
            Ok(bytes) => Ok(story.verify(&bytes).ok().map(|_| bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(TableError::Io(e)),
        }
    }

    /// Verifies `bytes` and writes them atomically (temp file, then rename).
    pub fn put(&self, story: Story, bytes: &[u8]) -> Result<(), TableError> {
        story.verify(bytes)?;
        fs::create_dir_all(&self.dir)?;
        let mut tmp = tempfile::NamedTempFile::new_in(&self.dir)?;
        tmp.write_all(bytes)?;
        tmp.flush()?;
        tmp.persist(self.path_for(story))
            .map_err(|e| TableError::Io(e.error))?;
        Ok(())
    }
}
