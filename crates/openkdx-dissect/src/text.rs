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
        Message::CreateNewsgroup(m) => newsgroup_summary(&m.group),
        Message::NewsgroupInfo(m) => newsgroup_summary(&m.group),
        Message::SetNewsgroup(m) => newsgroup_summary(&m.group),
        Message::DeleteNewsgroup(m) => format!("group {}", m.group_id),
        Message::GetNewsgroup(m) => format!("group {}", m.group_id),
        Message::GetNewsMessages(m) => format!("group {}", m.group_id),
        Message::GetNewsMessage(m) => format!("group {} message {}", m.group_id, m.message_id),
        Message::DeleteNewsMessage(m) => {
            format!("group {} message {}", m.group_id, m.message_id)
        }
        Message::NewsgroupList(m) => format!(
            "{} groups: {}",
            m.groups.len(),
            m.groups
                .iter()
                .map(|g| format!(
                    "{} {} ({} messages, {})",
                    g.group_id,
                    quoted(&g.name),
                    g.message_count,
                    time_1900(g.time)
                ))
                .collect::<Vec<_>>()
                .join("; ")
        ),
        Message::NewsMessages(m) => format!(
            "group {} permissions {:#x} {} messages: {}",
            m.group_id,
            m.permissions,
            m.messages.len(),
            m.messages
                .iter()
                .map(news_message_summary)
                .collect::<Vec<_>>()
                .join("; ")
        ),
        Message::NewsMessage(m) => news_message_summary(&m.message),
        Message::PostNewsMessage(m) => format!(
            "group {} subject {} body {} bytes",
            m.group_id,
            quoted(&m.subject),
            m.body.len()
        ),
        Message::NewsMessagePosted(m) => format!(
            "group {} message {} now {} messages",
            m.group_id, m.message_id, m.message_count
        ),
        Message::NewsMessageDeleted(m) => format!(
            "group {} message {} now {} messages",
            m.group_id, m.message_id, m.message_count
        ),
        Message::GetFileList(m) => format!("path {}", path(&m.path)),
        Message::GetFileFolderInfo(m) => format!("path {}", path(&m.path)),
        Message::LaunchFile(m) => format!("path {}", path(&m.path)),
        Message::FileList(m) => format!(
            "{} entries: {}",
            m.entries.len(),
            m.entries
                .iter()
                .map(|e| format!(
                    "{} ({}{} bytes{})",
                    quoted(&e.name),
                    if e.is_folder() { "folder, " } else { "" },
                    e.size,
                    if e.kind.is_empty() {
                        String::new()
                    } else {
                        format!(", {}", quoted(&e.kind))
                    }
                ))
                .collect::<Vec<_>>()
                .join("; ")
        ),
        Message::CreateFolder(m) => format!(
            "name {} access {} owner {} in {}",
            quoted(&m.name),
            quoted(&m.access),
            quoted(&m.owner),
            path(&m.path)
        ),
        Message::CreateAlias(m) => move_items(&m.items),
        Message::MoveFileFolder(m) => move_items(&m.items),
        Message::MoveToTrash(m) => format!("from {} items {}", path(&m.folder), names(&m.names)),
        Message::DeleteFileFolder(m) => {
            format!("{} bytes (unhandled by Server 1.620)", m.ignored.len())
        }
        Message::RenameFileFolder(m) => {
            format!("{} bytes (unhandled by Server 1.620)", m.ignored.len())
        }
        Message::CopyFileFolder(m) => {
            format!("{} bytes (unhandled by Server 1.620)", m.ignored.len())
        }
        Message::FileInfo(m) => format!(
            "name {} kind {} size {} created {} modified {}",
            quoted(&m.name),
            quoted(&m.kind),
            m.size,
            time_1900(m.created),
            time_1900(m.modified)
        ),
        Message::SetFileInfo(m) => format!(
            "flags {:#x} path {} name {}",
            m.flags,
            path(&m.path),
            quoted(&m.name)
        ),
        Message::DownloadFile(m) => format!(
            "path {} data offset {} rsrc offset {} options {:#x}",
            path(&m.path),
            m.data_offset,
            m.rsrc_offset,
            m.options
        ),
        Message::DownloadInfo(m) => format!(
            "name {} type {} data {} bytes rsrc {} bytes modified {}",
            quoted(&m.strings.name),
            quoted(&m.strings.second),
            m.data_size,
            m.rsrc_size,
            time_1900(m.modified)
        ),
        Message::DownloadFileData(m) => format!("{} bytes", m.data.len()),
        Message::UploadFileData(m) => format!("{} bytes", m.data.len()),
        Message::UploadFile(m) => format!(
            "name {} into {} data {} bytes rsrc {} bytes options {:#x}",
            quoted(&m.strings.name),
            path(&m.folder),
            m.data_size,
            m.rsrc_size,
            m.options
        ),
        Message::UploadReply(m) => format!(
            "resume data at {} rsrc at {:#x}",
            m.data_offset, m.rsrc_offset
        ),
        Message::NotifySpeedLimit(m) => format!("{} bytes/s", m.bytes_per_second),
        Message::VolumeInfo(m) => format!(
            "name {} format {} total {} free {} items {} type {}",
            quoted(&m.name),
            quoted(&m.format),
            m.total,
            m.free,
            m.item_count,
            m.disk_type
        ),
        Message::SetVolumeInfo(m) => format!(
            "flags {:#x} name {} new name {} comments {}",
            m.flags,
            quoted(&m.name),
            quoted(&m.new_name),
            quoted(&m.comments)
        ),
        Message::GenerateCatalog(m) => match m.flags {
            Some(f) => format!("flags {f:#x}"),
            None => String::new(),
        },
        Message::GenerateCatalogStatus(m) => format!("flags {:#x} items {}", m.flags, m.items),
        Message::FileSearch(m) => format!("query {}", quoted(&m.query)),
        Message::FileSearchReply(m) => format!(
            "{} matches: {}",
            m.entries.len(),
            m.entries
                .iter()
                .map(|e| format!("{} ({} bytes)", path(&e.path), e.size))
                .collect::<Vec<_>>()
                .join("; ")
        ),
        Message::TrackerGroupList(m) => format!(
            "{} groups: {}",
            m.groups.len(),
            m.groups
                .iter()
                .map(|g| format!("{} ({} servers)", quoted(&g.name), g.servers))
                .collect::<Vec<_>>()
                .join("; ")
        ),
        Message::GetServerList(m) => format!("group {}", quoted(&m.group)),
        Message::TrackerServerList(m) => format!(
            "{} servers: {}",
            m.servers.len(),
            m.servers
                .iter()
                .map(|s| format!(
                    "{} at {} ({} users, since {})",
                    quoted(&s.name),
                    s.socket_addr(),
                    s.users,
                    time_1900(s.first_seen)
                ))
                .collect::<Vec<_>>()
                .join("; ")
        ),
        Message::TrackerSettingsInfo(m) => tracker_settings(&m.settings),
        Message::SetTrackerSettings(m) => tracker_settings(&m.settings),
        Message::TrackerStats(m) => format!(
            "started {} accepted {} datagrams {} hours {} connections {} servers {}",
            time_1900(m.start_time),
            m.connections_accepted,
            m.datagrams_received,
            m.hours,
            m.connections,
            m.servers
        ),
        Message::GetAccountClass(m) => format!("class {}", m.class_index),
        Message::AccountClassInfo(m) => class_summary(m),
        Message::SetAccountClass(m) => class_summary(&m.class),
        Message::AccountClassList(m) => format!(
            "{} classes: {}",
            m.classes.len(),
            m.classes
                .iter()
                .map(|c| format!("{} {}", c.class_index, quoted(&c.name)))
                .collect::<Vec<_>>()
                .join("; ")
        ),
        Message::CreateAccount(m) => account_summary(&m.account),
        Message::AccountInfo(m) => account_summary(&m.account),
        Message::SetAccount(m) => account_summary(&m.account),
        Message::DeleteAccount(m) => format!("login {}", quoted(&m.login)),
        Message::GetAccount(m) => format!("login {}", quoted(&m.login)),
        Message::AccountDeleted(m) => format!("login {}", quoted(&m.login)),
        Message::AccountList(m) => format!(
            "{} accounts: {}",
            m.accounts.len(),
            m.accounts
                .iter()
                .map(|a| format!(
                    "{} ({}, class {})",
                    quoted(&a.login),
                    quoted(&a.name),
                    a.class_index
                ))
                .collect::<Vec<_>>()
                .join("; ")
        ),
        Message::AccountChanged(m) => format!("{} bytes", m.ignored.len()),
        Message::ChangeOwnPassword(m) => format!("key {}", hex(&m.account_key)),
        Message::ConnectionMonitorInfo(m) => format!(
            "{} records: {}",
            m.records.len(),
            m.records
                .iter()
                .map(|c| if c.is_transfer() {
                    format!(
                        "transfer {} user {} {} {}/{} bytes at {} B/s",
                        c.transfer_id,
                        c.user_id,
                        path(&c.path),
                        c.bytes_done,
                        c.bytes_total,
                        c.rate
                    )
                } else {
                    format!(
                        "user {} nick {} login {} class {} from {}",
                        c.user_id,
                        quoted(&c.nick),
                        quoted(&c.login),
                        quoted(&c.account),
                        quoted(&c.address)
                    )
                })
                .collect::<Vec<_>>()
                .join("; ")
        ),
        Message::DisconnectUser(m) => format!(
            "user {} ban {} s options {:#x} message {}",
            m.user_id,
            m.ban_seconds,
            m.options,
            quoted(&m.message)
        ),
        Message::DisconnectMessage(m) => format!(
            "user {} flags {:#x} message {} name {}",
            m.user_id,
            m.flags,
            quoted(&m.message),
            quoted(&m.name)
        ),
        Message::GetUserInfo(m) => format!("user {}", m.user_id),
        Message::UserInfo(m) => format!(
            "user {} nick {} login {} {} from {} since {}",
            m.user_id,
            quoted(&m.nick),
            quoted(&m.login),
            quoted(&m.name_3f),
            quoted(&m.address),
            time_1900(m.login_time)
        ),
        Message::SendBroadcast(m) => quoted(&m.text),
        Message::RcvBroadcast(m) => format!("from user {} {}", m.user_id, quoted(&m.text)),
        Message::ServerSettingsInfo(m) => server_settings(&m.settings),
        Message::SetServerSettings(m) => server_settings(&m.settings),
        Message::ShutdownServer(m) => format!("mode {:#x} message {}", m.mode, quoted(&m.message)),
        Message::HistoryList(m) => format!(
            "{} records: {}",
            m.records.len(),
            m.records
                .iter()
                .map(history_record)
                .collect::<Vec<_>>()
                .join("; ")
        ),
        Message::HistoryAdded(m) => history_record(&m.record),
        Message::ProcessList(m) => format!(
            "memory {}/{} bytes, {} processes: {}",
            m.memory_used,
            m.memory_total,
            m.processes.len(),
            m.processes
                .iter()
                .map(|p| format!("{} {} ({})", p.pid, quoted(&p.command), quoted(&p.user)))
                .collect::<Vec<_>>()
                .join("; ")
        ),
        Message::ExitTerminateProcess(m) => format!("flags {:#x} pid {}", m.flags, m.pid),
        Message::CloseViewDisplay(m) => format!("{} bytes", m.ignored.len()),
        Message::ViewDisplayParams(m) => format!("flags {:#x}", m.flags),
        Message::ViewDisplayUpdate(m) => format!(
            "screen {}x{} rect ({}, {})-({}, {}) {} pixel bytes",
            m.screen_width,
            m.screen_height,
            m.left,
            m.top,
            m.right,
            m.bottom,
            m.pixels.len()
        ),
        Message::ViewDisplayMouse(m) => format!(
            "kind {} button {} at ({}, {}) wheel {} modifiers {:#x}",
            m.kind, m.button, m.x, m.y, m.wheel, m.modifiers
        ),
        Message::ViewDisplayKeyboard(m) => format!(
            "kind {} character {:#x} key {} modifiers {:#x}",
            m.kind, m.character, m.key_code, m.modifiers
        ),
        Message::StartVoiceReply(m) => format!("result {:#x}", m.result),
        Message::VoiceFormat(m) => format!(
            "{} Hz {} {} ch {} bit buffer {} ms",
            m.sample_rate,
            String::from_utf8_lossy(&m.codec),
            m.channels,
            m.bits_per_sample,
            m.buffer
        ),
        Message::VoiceData(m) => format!("{} bytes", m.audio.len()),
        Message::CreateFolderAccess(m) => folder_access(&m.item),
        Message::FolderAccessInfo(m) => folder_access(&m.item),
        Message::SetFolderAccess(m) => folder_access(&m.item),
        Message::DeleteFolderAccess(m) => format!("item {}", quoted(&m.name)),
        Message::FolderAccessDeleted(m) => format!("item {}", quoted(&m.name)),
        Message::GetFolderAccess(m) => format!("item {}", quoted(&m.name)),
        Message::GetFolderAccessList(m) => match m.flags() {
            Some(f) => format!("flags {f:#x}"),
            None => String::new(),
        },
        Message::FolderAccessList(m) => format!(
            "{} items: {}",
            m.items.len(),
            m.items
                .iter()
                .map(|i| format!(
                    "{} (icon {:#x}, flags {:#x})",
                    quoted(&i.name),
                    i.icon,
                    i.flags
                ))
                .collect::<Vec<_>>()
                .join("; ")
        ),
        Message::OpenProcessMonitor(_)
        | Message::CloseProcessMonitor(_)
        | Message::GetProcessList(_)
        | Message::OpenViewDisplay(_)
        | Message::StartVoice(_)
        | Message::OpenConnectionMonitor(_)
        | Message::CloseConnectionMonitor(_)
        | Message::GetConnectionMonitorInfo(_)
        | Message::GetServerSettings(_)
        | Message::SetServerSettingsDone(_)
        | Message::UpgradeServerSoftware(_)
        | Message::ServerHistoryJoin(_)
        | Message::ServerHistoryLeave(_)
        | Message::GetAccountClassList(_)
        | Message::GetAccountList(_)
        | Message::ChangeOwnPasswordDone(_)
        | Message::GetTrackerGroupList(_)
        | Message::GetTrackerSettings(_)
        | Message::SetTrackerSettingsDone(_)
        | Message::GetTrackerStatistics(_)
        | Message::UploadFileComplete(_)
        | Message::EmptyTrash(_)
        | Message::GetNewsgroupList(_)
        | Message::CreatePrivateChat(_)
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

fn folder_access(f: &openkdx_proto::msg::misc::FolderAccess) -> String {
    format!(
        "item {} owner {} description {} icon {:#x} flags {:#x}{}",
        quoted(&f.name),
        quoted(&f.owner),
        quoted(&f.description),
        f.icon,
        f.flags,
        if f.current_name.is_empty() {
            String::new()
        } else {
            format!(" (was {})", quoted(&f.current_name))
        }
    )
}

fn server_settings(s: &openkdx_proto::msg::admin::ServerSettings) -> String {
    let sections: Vec<String> = (0..s.table.len())
        .filter_map(|i| s.section(i).map(|d| format!("{i}: {} bytes", d.len())))
        .collect();
    let name = s
        .main()
        .map(|m| quoted(&m.server_name))
        .unwrap_or_else(|| "?".into());
    format!(
        "version {} name {} sections [{}]",
        s.version,
        name,
        sections.join(", ")
    )
}

fn history_record(r: &openkdx_proto::msg::admin::HistoryRecord) -> String {
    format!(
        "{} {} user {} nick {} login {} from {} detail {} path {}",
        time_1900(r.time),
        r.kind.name().unwrap_or("unknown kind"),
        r.user_id,
        quoted(&r.nick),
        quoted(&r.login),
        quoted(&r.address),
        quoted(&r.detail),
        path(&r.path)
    )
}

fn privileges(p: &openkdx_proto::msg::accounts::Privileges) -> String {
    let names: Vec<&str> = p.set_bits().iter().map(|b| b.name()).collect();
    format!("[{}]", names.join(", "))
}

fn class_summary(c: &openkdx_proto::msg::accounts::AccountClassInfo) -> String {
    format!(
        "class {} {} flood {} speed {} colour {:02x}{:02x}{:02x} editor {} privileges {}",
        c.class_index,
        quoted(&c.name),
        c.flood_protection,
        c.speed_limit,
        c.color.r,
        c.color.g,
        c.color.b,
        quoted(&c.editor_login),
        privileges(&c.privileges)
    )
}

fn account_summary(a: &openkdx_proto::msg::accounts::Account) -> String {
    format!(
        "login {} name {} class {} editor {} password {} privileges {} mask {}",
        quoted(&a.login),
        quoted(&a.name),
        a.class_index,
        quoted(&a.editor),
        if a.key == [0; 32] {
            "unchanged/none"
        } else {
            "set"
        },
        privileges(&a.privileges),
        privileges(&a.privilege_mask)
    )
}

fn tracker_settings(s: &openkdx_proto::msg::tracker::TrackerSettings) -> String {
    let present: Vec<String> = s
        .sections
        .iter()
        .enumerate()
        .filter_map(|(i, d)| d.as_ref().map(|d| format!("{i}: {} bytes", d.len())))
        .collect();
    format!("version {} sections [{}]", s.version, present.join(", "))
}

/// A path as `a/b/c`, or `/` for the root.
pub fn path(p: &openkdx_proto::msg::common::FilePath) -> String {
    if p.is_root() {
        "/".to_string()
    } else {
        p.to_string()
    }
}

fn names(p: &openkdx_proto::msg::common::FilePath) -> String {
    let v: Vec<String> = p.names.iter().map(|n| quoted(&n.name)).collect();
    format!("[{}]", v.join(", "))
}

fn move_items(m: &openkdx_proto::msg::files::MoveItems) -> String {
    format!(
        "from {} to {} items {}",
        path(&m.source),
        path(&m.destination),
        names(&m.names)
    )
}

fn newsgroup_summary(g: &openkdx_proto::msg::news::NewsgroupInfo) -> String {
    format!(
        "group {} name {} description {} {} messages limit {} KiB classes read {:#010x} post {:#010x}",
        g.group_id,
        quoted(&g.name),
        quoted(&g.description),
        g.message_count,
        g.size_limit_kib,
        g.read_classes,
        g.post_classes
    )
}

fn news_message_summary(m: &openkdx_proto::msg::news::NewsMessage) -> String {
    format!(
        "message {} in group {} by {} ({}) subject {} body {} bytes at {}",
        m.message_id,
        m.group_id,
        quoted(&m.poster),
        quoted(&m.login),
        quoted(&m.subject),
        m.body.len(),
        time_1900(m.time)
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
