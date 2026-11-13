//! Text and JSON output.

use std::fmt::Display;

use openkdx_client::connection::Event;
use openkdx_proto::Message;
use openkdx_proto::msg::common::UserRecord;
use serde_json::{Value, json};

pub fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

/// A KDX time as `YYYY-MM-DD HH:MM:SS` UTC.
pub fn time_string(t: openkdx_proto::msg::common::Time1900) -> String {
    let ms = t.to_unix_ms();
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
    pub fn event(&self, event: &Event, users: &[UserRecord]) {
        let nick = |id: u32| -> String {
            users
                .iter()
                .find(|u| u.user_id == id)
                .map(|u| lossy(&u.nick))
                .unwrap_or_else(|| format!("#{id}"))
        };
        match event {
            Event::Message(m) => match &**m {
                Message::UserJoined(j) => self.record(
                    "joined",
                    &[
                        ("user_id", json!(j.user.user_id)),
                        ("nick", json!(lossy(&j.user.nick))),
                    ],
                ),
                Message::UserDeparted(d) => self.record(
                    "departed",
                    &[
                        ("user_id", json!(d.user_id)),
                        ("nick", json!(nick(d.user_id))),
                    ],
                ),
                Message::ReceiveChatText(t) => {
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
                Message::ChatTopicChanged(t) => self.record(
                    "topic",
                    &[
                        ("chat_id", json!(t.chat_id)),
                        ("nick", json!(lossy(&t.nick))),
                        ("topic", json!(lossy(&t.topic))),
                    ],
                ),
                Message::ChatUserJoined(j) => self.record(
                    "chat-joined",
                    &[
                        ("chat_id", json!(j.chat_id)),
                        ("nick", json!(nick(j.user_id))),
                    ],
                ),
                Message::ChatUserDeparted(j) => self.record(
                    "chat-departed",
                    &[
                        ("chat_id", json!(j.chat_id)),
                        ("nick", json!(nick(j.user_id))),
                    ],
                ),
                Message::IncomingPrivateMessage(p) => self.record(
                    "pm",
                    &[
                        ("user_id", json!(p.user_id)),
                        ("nick", json!(nick(p.user_id))),
                        ("text", json!(lossy(&p.text))),
                    ],
                ),
                Message::RcvBroadcast(b) => self.record(
                    "broadcast",
                    &[
                        ("user_id", json!(b.user_id)),
                        ("nick", json!(nick(b.user_id))),
                        ("text", json!(lossy(&b.text))),
                    ],
                ),
                Message::DisconnectMessage(d) => self.record(
                    "disconnect",
                    &[
                        ("banned", json!(d.flags & 2 != 0)),
                        ("shutdown", json!(d.flags & 4 != 0)),
                        ("by", json!(lossy(&d.name))),
                        ("message", json!(lossy(&d.message))),
                    ],
                ),
                Message::ServerGreeting(g) => {
                    self.record("greeting", &[("text", json!(lossy(&g.text)))])
                }
                Message::HistoryAdded(h) => self.record(
                    "history",
                    &[
                        ("time", json!(h.record.time.0)),
                        ("event", json!(h.record.kind.name().unwrap_or("?"))),
                        ("nick", json!(lossy(&h.record.nick))),
                        ("login", json!(lossy(&h.record.login))),
                        ("detail", json!(lossy(&h.record.detail))),
                    ],
                ),
                Message::NewsMessagePosted(p) => self.record(
                    "news-posted",
                    &[
                        ("group_id", json!(p.group_id)),
                        ("message_id", json!(p.message_id)),
                    ],
                ),
                other => {
                    if !self.quiet {
                        self.record("message", &[("type", json!(other.kind().to_string()))]);
                    }
                }
            },
            Event::Undecodable { kind, error, .. } => self.record(
                "undecodable",
                &[
                    ("type", json!(kind.to_string())),
                    ("error", json!(error.to_string())),
                ],
            ),
            Event::Disconnected(d) => self.record(
                "disconnected",
                &[
                    (
                        "message",
                        json!(d.message.as_ref().map(|m| lossy(&m.message))),
                    ),
                    ("error", json!(d.error.as_ref().map(|e| e.to_string()))),
                ],
            ),
        }
    }
}

fn plain(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}
