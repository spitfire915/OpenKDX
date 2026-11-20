//! Text and JSON output.

use std::fmt::Display;

use kdx_client::Notice;
use kdx_proto::msg::UserRecord;
use serde_json::{Value, json};

pub fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// Milliseconds from 1900-01-01 to the Unix epoch.
const UNIX_EPOCH_OFFSET_MS: i64 = 2_208_988_800_000;

/// A KDX time (milliseconds since 1900) as `YYYY-MM-DD HH:MM:SS` UTC.
pub fn time_string(ms_since_1900: u64) -> String {
    let ms = ms_since_1900 as i64 - UNIX_EPOCH_OFFSET_MS;
    let secs = ms.div_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    // Civil-from-days (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}:{:02}",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

/// `0x0654` → `1.620`.
pub fn version_string(v: u16) -> String {
    format!("{}.{:03}", v / 1000, v % 1000)
}

#[derive(Debug, Clone, Copy)]
pub struct Out {
    pub json: bool,
    pub quiet: bool,
}

impl Out {
    /// A note for a human; nothing in JSON mode or when quiet.
    pub fn note(&self, s: impl Display) {
        if !self.json && !self.quiet {
            println!("{s}");
        }
    }

    /// One record: key/value lines, or one JSON object.
    pub fn record(&self, kind: &str, pairs: &[(&str, Value)]) {
        if self.json {
            let mut obj = serde_json::Map::new();
            obj.insert("kind".into(), kind.into());
            for (k, v) in pairs {
                obj.insert((*k).into(), v.clone());
            }
            println!("{}", Value::Object(obj));
        } else {
            for (k, v) in pairs {
                println!("{k}: {}", plain(v));
            }
        }
    }

    /// Rows: aligned text columns, or one JSON object per row.
    pub fn table(&self, kind: &str, headers: &[&str], rows: &[Vec<Value>]) {
        if self.json {
            for row in rows {
                let mut obj = serde_json::Map::new();
                obj.insert("kind".into(), kind.into());
                for (h, v) in headers.iter().zip(row) {
                    obj.insert((*h).into(), v.clone());
                }
                println!("{}", Value::Object(obj));
            }
            return;
        }
        let cells: Vec<Vec<String>> = rows.iter().map(|r| r.iter().map(plain).collect()).collect();
        let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
        for row in &cells {
            for (i, c) in row.iter().enumerate() {
                widths[i] = widths[i].max(c.chars().count());
            }
        }
        let fmt = |cols: Vec<String>| {
            cols.iter()
                .enumerate()
                .map(|(i, c)| format!("{:<w$}", c, w = widths[i]))
                .collect::<Vec<_>>()
                .join("  ")
                .trim_end()
                .to_string()
        };
        if !self.quiet {
            println!("{}", fmt(headers.iter().map(|h| h.to_string()).collect()));
        }
        for row in cells {
            println!("{}", fmt(row));
        }
    }

    pub fn user_row(user: &UserRecord) -> Vec<Value> {
        vec![
            json!(user.user_id),
            json!(lossy(&user.nick)),
            json!(user.status),
            json!(lossy(&user.info)),
        ]
    }

    /// Print an unsolicited message the way a chat window would.
    pub fn event(&self, notice: &Notice, users: &[UserRecord]) {
        let nick = |id: u32| -> String {
            users
                .iter()
                .find(|u| u.user_id == id)
                .map(|u| lossy(&u.nick))
                .unwrap_or_else(|| format!("#{id}"))
        };
        match notice {
            Notice::UserJoined(u) => self.record(
                "joined",
                &[
                    ("user_id", json!(u.user_id)),
                    ("nick", json!(lossy(&u.nick))),
                ],
            ),
            Notice::UserChanged(u) => self.record(
                "changed",
                &[
                    ("user_id", json!(u.user_id)),
                    ("nick", json!(lossy(&u.nick))),
                ],
            ),
            Notice::UserDeparted(d) => self.record(
                "departed",
                &[
                    ("user_id", json!(d.user_id)),
                    ("nick", json!(nick(d.user_id))),
                ],
            ),
            Notice::ChatText(t) => {
                if self.json {
                    self.record(
                        "chat",
                        &[
                            ("chat_id", json!(t.chat_id)),
                            ("user_id", json!(t.user_id)),
                            ("nick", json!(nick(t.user_id))),
                            ("action", json!(t.is_action())),
                            ("text", json!(lossy(&t.text))),
                        ],
                    );
                } else if t.is_action() {
                    println!("* {} {}", nick(t.user_id), lossy(&t.text));
                } else {
                    println!("<{}> {}", nick(t.user_id), lossy(&t.text));
                }
            }
            Notice::ChatTopicChanged(t) => self.record(
                "topic",
                &[
                    ("chat_id", json!(t.chat_id)),
                    ("nick", json!(lossy(&t.nick))),
                    ("topic", json!(lossy(&t.topic))),
                ],
            ),
            Notice::ChatUserJoined(j) => self.record(
                "chat-joined",
                &[
                    ("chat_id", json!(j.chat_id)),
                    ("nick", json!(nick(j.user_id))),
                ],
            ),
            Notice::ChatUserDeparted(j) => self.record(
                "chat-departed",
                &[
                    ("chat_id", json!(j.chat_id)),
                    ("nick", json!(nick(j.user_id))),
                ],
            ),
            Notice::ChatInvitation(i) => self.record(
                "chat-invitation",
                &[
                    ("chat_id", json!(i.chat_id)),
                    ("nick", json!(nick(i.user_id))),
                    ("message", json!(lossy(&i.message))),
                ],
            ),
            Notice::ChatDeleted(d) => self.record("chat-deleted", &[("chat_id", json!(d.chat_id))]),
            Notice::PrivateMessage(p) => self.record(
                "pm",
                &[
                    ("user_id", json!(p.user_id)),
                    ("nick", json!(nick(p.user_id))),
                    ("text", json!(lossy(&p.text))),
                ],
            ),
            Notice::Broadcast(b) => self.record(
                "broadcast",
                &[
                    ("user_id", json!(b.user_id)),
                    ("nick", json!(nick(b.user_id))),
                    ("text", json!(lossy(&b.text))),
                ],
            ),
            Notice::Disconnect(d) => self.record(
                "disconnect",
                &[
                    ("banned", json!(d.flags & 2 != 0)),
                    ("shutdown", json!(d.flags & 4 != 0)),
                    ("by", json!(lossy(&d.name))),
                    ("message", json!(lossy(&d.message))),
                ],
            ),
            Notice::ServerGreeting(g) => {
                self.record("greeting", &[("text", json!(lossy(&g.text)))])
            }
            Notice::HistoryAdded(h) => self.record(
                "history",
                &[
                    ("time", json!(h.time)),
                    ("event", json!(h.kind.title().unwrap_or("?"))),
                    ("nick", json!(lossy(&h.nick))),
                    ("login", json!(lossy(&h.login))),
                    ("detail", json!(lossy(&h.detail))),
                ],
            ),
            Notice::NewsPosted(p) => self.record(
                "news-posted",
                &[
                    ("group_id", json!(p.group_id)),
                    ("message_id", json!(p.message_id)),
                ],
            ),
            Notice::NewsDeleted(p) => self.record(
                "news-deleted",
                &[
                    ("group_id", json!(p.group_id)),
                    ("message_id", json!(p.message_id)),
                ],
            ),
            Notice::Other(header, _) => {
                if !self.quiet {
                    self.record("message", &[("type", json!(header.kind.to_string()))]);
                }
            }
            other => {
                if !self.quiet {
                    self.record("message", &[("type", json!(notice_name(other)))]);
                }
            }
        }
    }
}

/// The variant name of a notice this output has no special form for.
fn notice_name(notice: &Notice) -> String {
    let debug = format!("{notice:?}");
    debug
        .split(|c: char| !c.is_ascii_alphanumeric())
        .next()
        .unwrap_or("notice")
        .to_string()
}

fn plain(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}
