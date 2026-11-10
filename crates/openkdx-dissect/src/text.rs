//! Human-readable rendering of records: one-line summaries, hexdumps,
//! quoted byte strings, times.

use std::fmt::Write;

use openkdx_proto::handshake::ProofStatus;
use openkdx_proto::message_type::MessageType;
use openkdx_proto::msg::common::Time1900;
use openkdx_proto::msg::{Message, error_name};

use crate::connection::Record;

/// Bytes as a double-quoted string, printable ASCII kept and the rest as `\xNN`.
pub fn quoted(bytes: &[u8]) -> String {
    let mut s = String::from("\"");
    for &b in bytes {
        match b {
            b'"' => s.push_str("\\\""),
            b'\\' => s.push_str("\\\\"),
            0x20..=0x7e => s.push(b as char),
            _ => {
                let _ = write!(s, "\\x{b:02x}");
            }
        }
    }
    s.push('"');
    s
}

pub fn hex(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

/// A classic 16-bytes-per-line hexdump, each line prefixed with `indent`.
pub fn hexdump(data: &[u8], indent: &str) -> String {
    let mut out = String::new();
    for (i, chunk) in data.chunks(16).enumerate() {
        let _ = write!(out, "{indent}{:04x}  ", i * 16);
        for j in 0..16 {
            match chunk.get(j) {
                Some(b) => {
                    let _ = write!(out, "{b:02x} ");
                }
                None => out.push_str("   "),
            }
        }
        out.push(' ');
        for &b in chunk {
            out.push(if (0x20..=0x7e).contains(&b) {
                b as char
            } else {
                '.'
            });
        }
        out.push('\n');
    }
    out
}

/// A KDX time as `YYYY-MM-DD HH:MM:SS.mmm UTC`.
pub fn time_1900(t: Time1900) -> String {
    let unix_ms = t.to_unix_ms();
    let (secs, ms) = (unix_ms.div_euclid(1000), unix_ms.rem_euclid(1000));
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (h, m, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (y, mo, d) = civil_from_days(days);
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{m:02}:{s:02}.{ms:03} UTC")
}

// Howard Hinnant's days-to-civil algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// One line describing a record, without the frame and direction prefix.
pub fn summary(record: &Record) -> String {
    match record {
        Record::Hello { hello, .. } => format!(
            "hello: {} version {} sub-version {:#04x} challenge {} nonce {} name {}",
            String::from_utf8_lossy(&hello.sub_protocol.tag()),
            hello.version,
            hello.sub_version,
            hello.challenge,
            hex(&hello.nonce),
            quoted(&hello.name)
        ),
        Record::Proof {
            proof,
            auth_ok,
            challenge_ok,
            ..
        } => {
            let status = match proof.status {
                ProofStatus::Ok => "Ok".to_string(),
                other => format!("{other:?}"),
            };
            format!(
                "proof: status {status} auth {} {} response {} {}",
                hex(&proof.auth),
                check(*auth_ok),
                quoted(&proof.response),
                check(*challenge_ok)
            )
        }
        Record::Message {
            header, message, ..
        } => {
            let mut s = format!(
                "txid {:08x} {}{} len {}",
                header.txid,
                MessageType(header.kind),
                if header.reply { " reply" } else { "" },
                header.len
            );
            if let Ok(m) = message {
                let detail = message_summary(m);
                if !detail.is_empty() {
                    s.push_str("  ");
                    s.push_str(&detail);
                }
            }
            s
        }
        Record::DataHeader { header, .. } => format!(
            "txid {:08x} {}{} data header, {} bytes follow",
            header.txid,
            MessageType(header.kind),
            if header.reply { " reply" } else { "" },
            header.len
        ),
        Record::Data { len, .. } => format!("transfer data: {len} bytes"),
        Record::Stopped { reason, .. } => format!("stopped: {reason}"),
    }
}

fn check(v: Option<bool>) -> &'static str {
    match v {
        Some(true) => "(ok)",
        Some(false) => "(MISMATCH)",
        None => "(unchecked)",
    }
}

/// The fields of a message worth a line. Grows as message types are modeled.
pub fn message_summary(m: &Message) -> String {
    match m {
        Message::Status(s) => {
            if s.is_error() {
                format!(
                    "error {:#x} ({}) for request {}",
                    s.code,
                    error_name(s.code).unwrap_or("not in the table"),
                    s.request_type_or_value
                )
            } else {
                format!("ok, value {}", s.request_type_or_value)
            }
        }
        Message::Unknown { .. } => String::new(),
    }
}
