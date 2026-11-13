//! `History/`: the server's tab-separated history logs. Append-only in the
//! original format. See `docs/formats/history.md`.
//!
//! One file per server start, `KDX yy-mm-dd hh-mm-ss AM.txt`, UTF-8 BOM
//! first, then one line per record:
//! `yy/mm/dd hh:mm:ss PM<TAB>event<TAB>nick<TAB>login<TAB>detail<TAB>0[<TAB>n…]<LF>`.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use openkdx_proto::msg::common::Time1900;

use crate::civil::Civil;
use crate::error::{LegacyError, Result};
use crate::store::LegacyDir;

pub const BOM: [u8; 3] = [0xef, 0xbb, 0xbf];

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HistoryFile {
    pub bom: bool,
    pub lines: Vec<HistoryLine>,
}

/// One record, with every column as the file holds it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HistoryLine {
    /// `yy/mm/dd hh:mm:ss AM|PM`.
    pub time: Vec<u8>,
    /// The event's name, as 183 History Added names it (`Login`,
    /// `Disconnected`, `Upload File`, `Create Account`, …).
    pub event: Vec<u8>,
    pub nick: Vec<u8>,
    pub login: Vec<u8>,
    /// The address, path, account or file the event concerns; may be empty.
    pub detail: Vec<u8>,
    /// Trailing numeric columns: `0`, and for transfers two more
    /// (bytes done, bytes total).
    pub extra: Vec<Vec<u8>>,
}

impl HistoryLine {
    pub fn new(time: Time1900, event: &[u8], nick: &[u8], login: &[u8], detail: &[u8]) -> Self {
        HistoryLine {
            time: format_history_time(time).into_bytes(),
            event: event.to_vec(),
            nick: nick.to_vec(),
            login: login.to_vec(),
            detail: detail.to_vec(),
            extra: vec![b"0".to_vec()],
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        for (i, col) in [
            &self.time,
            &self.event,
            &self.nick,
            &self.login,
            &self.detail,
        ]
        .into_iter()
        .chain(self.extra.iter())
        .enumerate()
        {
            if i > 0 {
                out.push(b'\t');
            }
            out.extend_from_slice(col);
        }
        out.push(b'\n');
        out
    }
}

impl HistoryFile {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        if self.bom {
            out.extend_from_slice(&BOM);
        }
        for l in &self.lines {
            out.extend(l.to_bytes());
        }
        out
    }
}

pub fn format_history_time(t: Time1900) -> String {
    let c = Civil::from_time(t);
    let (h, ampm) = c.twelve_hour();
    format!(
        "{:02}/{:02}/{:02} {:02}:{:02}:{:02} {}",
        c.year.rem_euclid(100),
        c.month,
        c.day,
        h,
        c.minute,
        c.second,
        ampm
    )
}

/// `KDX yy-mm-dd hh-mm-ss PM.txt`, from the server's start time.
pub fn history_file_name(started: Time1900) -> String {
    let c = Civil::from_time(started);
    let (h, ampm) = c.twelve_hour();
    format!(
        "KDX {:02}-{:02}-{:02} {:02}-{:02}-{:02} {}.txt",
        c.year.rem_euclid(100),
        c.month,
        c.day,
        h,
        c.minute,
        c.second,
        ampm
    )
}

pub fn parse_history(bytes: &[u8]) -> Result<HistoryFile> {
    let (bom, rest) = match bytes.strip_prefix(&BOM) {
        Some(r) => (true, r),
        None => (false, bytes),
    };
    let mut lines = Vec::new();
    for raw in rest.split(|&b| b == b'\n') {
        if raw.is_empty() {
            continue;
        }
        let cols: Vec<&[u8]> = raw.split(|&b| b == b'\t').collect();
        if cols.len() < 6 {
            return Err(LegacyError::Invalid("History: fewer than six columns"));
        }
        lines.push(HistoryLine {
            time: cols[0].to_vec(),
            event: cols[1].to_vec(),
            nick: cols[2].to_vec(),
            login: cols[3].to_vec(),
            detail: cols[4].to_vec(),
            extra: cols[5..].iter().map(|c| c.to_vec()).collect(),
        });
    }
    if !rest.is_empty() && !rest.ends_with(b"\n") {
        return Err(LegacyError::Invalid("History: last line not terminated"));
    }
    Ok(HistoryFile { bom, lines })
}

impl LegacyDir {
    /// Appends to this run's file (named after `history_started`), creating
    /// `History/` and the file (with its BOM) as needed. Returns the path.
    pub fn append_history(&self, line: &HistoryLine) -> Result<PathBuf> {
        let dir = self.history_dir();
        fs::create_dir_all(&dir).map_err(|e| LegacyError::io(&dir, e))?;
        let name = history_file_name(self.history_started());
        let path = dir.join(name);
        let fresh = !path.exists();
        let mut f = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .map_err(|e| LegacyError::io(&path, e))?;
        if fresh {
            f.write_all(&BOM).map_err(|e| LegacyError::io(&path, e))?;
        }
        f.write_all(&line.to_bytes())
            .map_err(|e| LegacyError::io(&path, e))?;
        Ok(path)
    }
}
