//! The live store: an original KDX data directory, backed up once and
//! rewritten atomically in the original formats (design spec §5.2).

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::error::{LegacyError, Result};

/// The files the original server and tracker keep in their directory.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServerFile {
    Accounts,
    Settings,
    News,
    Tracker,
}

impl ServerFile {
    pub const ALL: [ServerFile; 4] = [
        ServerFile::Accounts,
        ServerFile::Settings,
        ServerFile::News,
        ServerFile::Tracker,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ServerFile::Accounts => "Accounts.dat",
            ServerFile::Settings => "KDXServer.stg",
            ServerFile::News => "News.dat",
            ServerFile::Tracker => "KDXTrackerServ.stg",
        }
    }
}

/// An original data directory. Opening it copies every known file that has
/// no `<name>.orig` yet to that name, once; an existing `.orig` is never
/// touched, so the operator can always roll back to what Haxial's program
/// last wrote before OpenKDX took over.
#[derive(Debug, Clone)]
pub struct LegacyDir {
    root: PathBuf,
}

impl LegacyDir {
    pub fn open(root: impl Into<PathBuf>) -> Result<LegacyDir> {
        let root = root.into();
        let meta = fs::metadata(&root).map_err(|e| LegacyError::io(&root, e))?;
        if !meta.is_dir() {
            return Err(LegacyError::io(
                &root,
                std::io::Error::new(std::io::ErrorKind::NotADirectory, "not a directory"),
            ));
        }
        let dir = LegacyDir { root };
        for file in ServerFile::ALL {
            dir.backup_once(file)?;
        }
        Ok(dir)
    }

    /// Copies the live file to `<name>.orig` if the backup does not exist
    /// yet, through `<name>.orig.tmp` and a rename so a crash never leaves a
    /// partial backup that would stop a later complete one.
    fn backup_once(&self, f: ServerFile) -> Result<()> {
        let live = self.path(f);
        let orig = self.orig_path(f);
        if !live.is_file() || orig.exists() {
            return Ok(());
        }
        let tmp = self.root.join(format!("{}.orig.tmp", f.name()));
        let bytes = fs::read(&live).map_err(|e| LegacyError::io(&live, e))?;
        write_sync(&tmp, &bytes, unix_mode(&live))?;
        fs::rename(&tmp, &orig).map_err(|e| LegacyError::io(&orig, e))
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn path(&self, f: ServerFile) -> PathBuf {
        self.root.join(f.name())
    }

    pub fn orig_path(&self, f: ServerFile) -> PathBuf {
        self.root.join(format!("{}.orig", f.name()))
    }

    pub fn history_dir(&self) -> PathBuf {
        self.root.join("History")
    }

    pub fn news_archives_dir(&self) -> PathBuf {
        self.root.join("News Archives")
    }

    pub fn bases_dir(&self) -> PathBuf {
        self.root.join("Bases")
    }

    /// The file's bytes, or `None` when it does not exist.
    pub fn read(&self, f: ServerFile) -> Result<Option<Vec<u8>>> {
        let path = self.path(f);
        match fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(LegacyError::io(path, e)),
        }
    }

    /// Writes `<name>.tmp` (replacing any stale one), then renames it over
    /// the live file, so a crash leaves either the old file or the new one.
    /// The live file's permissions are kept; a new file is created `0600`
    /// on Unix, as the original programs do.
    pub fn save(&self, f: ServerFile, bytes: &[u8]) -> Result<()> {
        self.backup_once(f)?;
        let live = self.path(f);
        let tmp = self.root.join(format!("{}.tmp", f.name()));
        write_sync(&tmp, bytes, unix_mode(&live))?;
        fs::rename(&tmp, &live).map_err(|e| LegacyError::io(&live, e))
    }
}

/// The live file's mode, or `0600` for a new one (what the originals use).
fn unix_mode(live: &Path) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(live)
            .map(|m| m.permissions().mode() & 0o777)
            .unwrap_or(0o600)
    }
    #[cfg(not(unix))]
    {
        let _ = live;
        0o600
    }
}

/// Creates or truncates `path` with `mode` from the start, writes, syncs.
fn write_sync(path: &Path, bytes: &[u8], mode: u32) -> Result<()> {
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(mode);
    }
    #[cfg(not(unix))]
    let _ = mode;
    let mut out = opts.open(path).map_err(|e| LegacyError::io(path, e))?;
    #[cfg(unix)]
    {
        // `mode` only applies to a newly created file; a stale one keeps its
        // old mode until told otherwise.
        use std::os::unix::fs::PermissionsExt;
        out.set_permissions(fs::Permissions::from_mode(mode))
            .map_err(|e| LegacyError::io(path, e))?;
    }
    out.write_all(bytes).map_err(|e| LegacyError::io(path, e))?;
    out.sync_all().map_err(|e| LegacyError::io(path, e))
}
