//! Human-readable rendering of records: one-line summaries, hexdumps,
//! quoted byte strings, times.

use std::fmt::Write;

use openkdx_proto::handshake::ProofStatus;
use openkdx_proto::message_type::MessageType;
use openkdx_proto::msg::common::{Time1900, UserRecord};
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
        Message::SetIdentity(m) => user_summary(&m.user),
        Message::UserChanged(m) => user_summary(&m.user),
        Message::UserJoined(m) => user_summary(&m.user),
        Message::UserList(m) => format!(
            "{} users: {}",
            m.users.len(),
            m.users
                .iter()
                .map(user_summary)
                .collect::<Vec<_>>()
                .join("; ")
        ),
        Message::UserDeparted(m) => format!("user {} kind {}", m.user_id, m.kind),
        Message::ServerInfo(m) => format!(
            "user {} version {} name {} description {} address {} transfer port {} modify settings {} session key {}",
            m.user_id,
            m.version,
            quoted(&m.name),
            quoted(&m.description),
            quoted(&m.address),
            m.file_transfer_port,
            m.can_modify_settings,
            hex(&m.session_key)
        ),
        Message::ServerIcon(m) => format!("{} bytes", m.icon.len()),
        Message::ServerGreeting(m) => quoted(&m.text),
        Message::SetMiniUserIcon(m) => format!("status {}", m.icon.status),
        Message::MiniUserIconChanged(m) => format!("user {} status {}", m.user_id, m.icon.status),
        Message::ExtraServerInfo(m) => format!(
            "started {} now {} uptime {} s os {:#x}/{} {}",
            time_1900(m.start_time),
            time_1900(m.now),
            m.uptime,
            m.os_kind,
            m.os_version,
            quoted(&m.os)
        ),
        Message::YourAddress(m) => quoted(&m.address),
        Message::Skpk(m) => format!(
            "port {} connection {} version {} key {}",
            m.dcc_port,
            m.connection_id,
            m.client_version,
            hex(&m.key)
        ),
        Message::LicenseReport(m) => format!("value {}", m.value),
        Message::DeleteChat(m) => format!("chat {}", m.chat_id),
        Message::LeaveChat(m) => format!("chat {}", m.chat_id),
        Message::GetChat(m) => format!("chat {}", m.chat_id),
        Message::GetChatDccAddresses(m) => format!("chat {}", m.chat_id),
        Message::JoinChat(m) => format!("chat {} key {:#x}", m.chat_id, m.key),
        Message::ChatWindowInfo(m) => window_info_summary(m),
        Message::CreatePrivateChatReply(m) => window_info_summary(&m.info),
        Message::ChatUserJoined(m) => format!("chat {} user {}", m.chat_id, m.user_id),
        Message::ChatUserDeparted(m) => format!("chat {} user {}", m.chat_id, m.user_id),
        Message::SendChatText(m) => format!(
            "chat {} {}{}",
            m.chat_id,
            if m.is_action() { "action " } else { "" },
            quoted(&m.text)
        ),
        Message::ReceiveChatText(m) => format!(
            "chat {} user {} {}{}",
            m.chat_id,
            m.user_id,
            if m.is_action() { "action " } else { "" },
            quoted(&m.text)
        ),
        Message::SetChatTopic(m) => format!("chat {} topic {}", m.chat_id, quoted(&m.topic)),
        Message::ChatTopicChanged(m) => format!(
            "chat {} user {} nick {} topic {} at {}",
            m.chat_id,
            m.user_id,
            quoted(&m.nick),
            quoted(&m.topic),
            time_1900(m.time)
        ),
        Message::ChatInvite(m) => format!(
            "chat {} users {:?} message {}",
            m.chat_id,
            m.user_ids,
            quoted(&m.message)
        ),
        Message::ChatInvitation(m) => format!(
            "chat {} key {:#x} from user {} name {}",
            m.chat_id,
            m.key,
            m.user_id,
            quoted(&m.name)
        ),
        Message::ChatList(m) => format!(
            "{} chats: {}",
            m.chats.len(),
            m.chats
                .iter()
                .map(|c| format!(
                    "{} {} ({}, {} members, flags {:#x})",
                    c.chat_id,
                    quoted(&c.name),
                    quoted(&c.description),
                    c.member_count,
                    c.flags
                ))
                .collect::<Vec<_>>()
                .join("; ")
        ),
        Message::CreateChat(m) => chat_record_summary(&m.chat),
        Message::ChatInfo(m) => chat_record_summary(&m.chat),
        Message::SetChat(m) => chat_record_summary(&m.chat),
        Message::ChatDeleted(m) => format!("chat {} by user {}", m.chat_id, m.user_id),
        Message::ChatDccAddresses(m) => m
            .addresses
            .iter()
            .map(|a| format!("user {} at {}", a.user_id, quoted(&a.address)))
            .collect::<Vec<_>>()
            .join("; "),
        Message::SendPrivateMessage(m) => format!("to user {} {}", m.user_id, quoted(&m.text)),
        Message::IncomingPrivateMessage(m) => {
            format!("from user {} {}", m.user_id, quoted(&m.text))
        }
        Message::CreatePrivateChat(_)
        | Message::MustJoinChat(_)
        | Message::GetChatList(_)
        | Message::Ping(_)
        | Message::Pong(_)
        | Message::RequestGreeting(_)
        | Message::GetExtraServerInfo(_)
        | Message::GetMyAddress(_)
        | Message::Unknown { .. } => String::new(),
    }
}

fn window_info_summary(m: &openkdx_proto::msg::chat::ChatWindowInfo) -> String {
    format!(
        "chat {} key {:#x} flags {:#x} name {} topic {} by {} members {:?} irc {} {}",
        m.chat_id,
        m.key,
        m.flags,
        quoted(&m.name),
        quoted(&m.topic),
        quoted(&m.topic_setter),
        m.members,
        quoted(&m.irc_server),
        quoted(&m.irc_channel)
    )
}

fn chat_record_summary(c: &openkdx_proto::msg::common::ChatRecord) -> String {
    format!(
        "chat {} key {:#x} flags {:#x} name {} description {} classes join {:#010x} send {:#010x} topic {:#010x} irc {} {}",
        c.chat_id,
        c.key,
        c.flags,
        quoted(&c.name),
        quoted(&c.description),
        c.join_classes,
        c.send_classes,
        c.topic_classes,
        quoted(&c.irc_server),
        quoted(&c.irc_channel)
    )
}

fn user_summary(u: &UserRecord) -> String {
    format!(
        "user {} nick {} info {} port {} icon {} bytes colours {:02x}{:02x}{:02x}/{:02x}{:02x}{:02x}/{:02x}{:02x}{:02x} flags {:#x}",
        u.user_id,
        quoted(&u.nick),
        quoted(&u.info),
        u.client_port,
        u.icon.len(),
        u.text_color.r,
        u.text_color.g,
        u.text_color.b,
        u.background_color.r,
        u.background_color.g,
        u.background_color.b,
        u.account_color.r,
        u.account_color.g,
        u.account_color.b,
        u.flags
    )
}
