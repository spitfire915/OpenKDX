//! `News Archives/<group>/<group> <yymmdd>.txt`: messages the server moved
//! out of a newsgroup that outgrew its size limit, as text. Append-only in
//! the original format. See `docs/formats/news-archives.md`.
//!
//! UTF-8 BOM, then per message: LF, sixty underscores, LF, the subject,
//! LF, `<poster>`, two spaces, `Fri, 13 Nov 2026, 06:17:32 PM`, LF, the
//! body as stored (CR line ends). No terminator after the last body.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use openkdx_proto::msg::common::Time1900;
use openkdx_proto::msg::news::NewsMessage;

use crate::civil::Civil;
use crate::error::{LegacyError, Result};
use crate::history::BOM;
use crate::store::LegacyDir;

const RULE: &[u8; 60] = b"____________________________________________________________";

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ArchiveEntry {
    pub subject: Vec<u8>,
    pub poster: Vec<u8>,
    /// `Fri, 13 Nov 2026, 06:17:32 PM`, as written.
    pub time: Vec<u8>,
    pub body: Vec<u8>,
}

impl ArchiveEntry {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.body.len() + 128);
        out.push(b'\n');
        out.extend_from_slice(RULE);
        out.push(b'\n');
        out.extend_from_slice(&self.subject);
        out.extend_from_slice(b"\n<");
        out.extend_from_slice(&self.poster);
        out.extend_from_slice(b">  ");
        out.extend_from_slice(&self.time);
        out.push(b'\n');
        out.extend_from_slice(&self.body);
        out
    }
}

pub fn format_archive_time(t: Time1900) -> String {
    let c = Civil::from_time(t);
    let (h, ampm) = c.twelve_hour();
    format!(
        "{}, {} {} {}, {:02}:{:02}:{:02} {}",
        c.weekday_name(),
        c.day,
        c.month_name(),
        c.year,
        h,
        c.minute,
        c.second,
        ampm
    )
}

/// `<group> <yymmdd>.txt`.
pub fn archive_file_name(group: &[u8], when: Time1900) -> String {
    let c = Civil::from_time(when);
    format!(
        "{} {:02}{:02}{:02}.txt",
        String::from_utf8_lossy(group),
        c.year.rem_euclid(100),
        c.month,
        c.day
    )
}

pub fn format_archive_entry(m: &NewsMessage) -> Vec<u8> {
    ArchiveEntry {
        subject: m.subject.clone(),
        poster: m.poster.clone(),
        time: format_archive_time(m.time).into_bytes(),
        body: m.body.clone(),
    }
    .to_bytes()
}

pub fn parse_news_archive(bytes: &[u8]) -> Result<Vec<ArchiveEntry>> {
    let rest = bytes.strip_prefix(&BOM).unwrap_or(bytes);
    if rest.is_empty() {
        return Ok(Vec::new());
    }
    let mut sep = vec![b'\n'];
    sep.extend_from_slice(RULE);
    sep.push(b'\n');
    if !rest.starts_with(&sep) {
        return Err(LegacyError::Invalid("News archive: no leading rule"));
    }
    let mut entries = Vec::new();
    let mut pos = sep.len();
    loop {
        let next = find(&rest[pos..], &sep).map(|i| pos + i);
        let chunk = &rest[pos..next.unwrap_or(rest.len())];
        let nl1 = chunk
            .iter()
            .position(|&b| b == b'\n')
            .ok_or(LegacyError::Invalid("News archive: no subject line"))?;
        let subject = &chunk[..nl1];
        let after = &chunk[nl1 + 1..];
        let nl2 = after
            .iter()
            .position(|&b| b == b'\n')
            .ok_or(LegacyError::Invalid("News archive: no poster line"))?;
        let head = &after[..nl2];
        let close = head
            .strip_prefix(b"<")
            .and_then(|h| find(h, b">  ").map(|i| (&h[..i], &h[i + 3..])))
            .ok_or(LegacyError::Invalid("News archive: poster line"))?;
        entries.push(ArchiveEntry {
            subject: subject.to_vec(),
            poster: close.0.to_vec(),
            time: close.1.to_vec(),
            body: after[nl2 + 1..].to_vec(),
        });
        match next {
            Some(n) => pos = n + sep.len(),
            None => break,
        }
    }
    Ok(entries)
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

impl LegacyDir {
    /// Appends the message to today's archive file of `group`, creating the
    /// directory and the file (with its BOM) as needed. Returns the path.
    pub fn append_news_archive(&self, group: &[u8], m: &NewsMessage) -> Result<PathBuf> {
        let dir = self
            .news_archives_dir()
            .join(String::from_utf8_lossy(group).as_ref());
        fs::create_dir_all(&dir).map_err(|e| LegacyError::io(&dir, e))?;
        let path = dir.join(archive_file_name(group, m.time));
        let fresh = !path.exists();
        let mut f = fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .map_err(|e| LegacyError::io(&path, e))?;
        if fresh {
            f.write_all(&BOM).map_err(|e| LegacyError::io(&path, e))?;
        }
        f.write_all(&format_archive_entry(m))
            .map_err(|e| LegacyError::io(&path, e))?;
        Ok(path)
    }
}
