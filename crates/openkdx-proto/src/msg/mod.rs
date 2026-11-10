//! Typed message payloads. One struct per message in the reference spec,
//! each implementing [`Payload`]; [`Message`] gathers them.
//!
//! Every payload decodes to exactly the bytes it came from: `ignored_*`,
//! `extra`, and `padding` bytes are kept as fields and written back. Bytes a
//! struct does not account for fail decoding with [`DecodeError::Trailing`].

pub mod accounts;
pub mod admin;
pub mod chat;
pub mod common;
pub mod files;
pub mod gconsole;
pub mod misc;
pub mod news;
pub mod session;
pub mod tracker;

use std::fmt;

use crate::message_type::MessageType;
use crate::wire::{DecodeError, Reader, Writer};

/// A message payload with a fixed type number.
pub trait Payload: Sized + fmt::Debug {
    const KIND: MessageType;

    /// Decodes the whole payload; trailing bytes are an error.
    fn decode(bytes: &[u8]) -> Result<Self, DecodeError>;

    fn encode(&self, w: &mut Writer);

    fn to_bytes(&self) -> Vec<u8> {
        let mut w = Writer::new();
        self.encode(&mut w);
        w.into_inner()
    }
}

/// The 16 bytes the server adds to an error [`Status`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusDetail {
    /// ASCII module tag packed into an integer: 0x54524e is `TRN`.
    pub module: u32,
    pub line: u32,
    pub extra: u32,
    pub ignored: u32,
}

/// 1 Status: the reply that carries a result code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    /// 0 for success, else an error code; see [`error_name`].
    pub code: u32,
    /// The request type in an error, or the value in a success.
    pub request_type_or_value: u32,
    /// Only in the server's 24-byte error form.
    pub detail: Option<StatusDetail>,
}

impl Status {
    pub fn is_error(&self) -> bool {
        self.code != 0
    }
}

impl Payload for Status {
    const KIND: MessageType = MessageType::STATUS;

    fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut r = Reader::new(bytes);
        let code = r.u32()?;
        let request_type_or_value = r.u32()?;
        let detail = if r.is_empty() {
            None
        } else {
            Some(StatusDetail {
                module: r.u32()?,
                line: r.u32()?,
                extra: r.u32()?,
                ignored: r.u32()?,
            })
        };
        r.finish()?;
        Ok(Status {
            code,
            request_type_or_value,
            detail,
        })
    }

    fn encode(&self, w: &mut Writer) {
        w.u32(self.code);
        w.u32(self.request_type_or_value);
        if let Some(d) = &self.detail {
            w.u32(d.module);
            w.u32(d.line);
            w.u32(d.extra);
            w.u32(d.ignored);
        }
    }
}

/// The meaning of an error code, from the server's table of 79 names.
pub fn error_name(code: u32) -> Option<&'static str> {
    Some(match code {
        0x10001 => "unknown error",
        0x10002 => "invalid value",
        0x10003 => "protocol breach",
        0x10004 => "unimplemented",
        0x10005 => "unsupported",
        0x10006 => "timed out",
        0x10007 => "aborted",
        0x10008 => "data version incompatible",
        0x10009 => "data format incompatible",
        0x1000a => "corrupt data",
        0x1000b => "out of range",
        0x1000c => "item does not exist",
        0x1000d => "item already exists",
        0x1000e => "limit cannot be exceeded",
        0x1000f => "locked",
        0x10010 => "data file missing",
        0x10011 => "resource missing",
        0x10012 => "process does not exist",
        0x10013 => "IPC queue full",
        0x10014 => "Access Denied (a missing privilege)",
        0x10015 => "hardware error",
        0x10016 => "hardware busy",
        0x10017 => "unknown view class",
        0x10018 => "view does not exist",
        0x10019 => "invalid object",
        0x1001a => "buffer too small",
        0x1001b => "unknown OS error",
        0x1001c => "not initialized",
        0x1001d => "re-entry not allowed",
        0x1001e => "invalid name",
        0x20064..=0x20066 => "memory errors",
        0x30064..=0x30065 => "file system errors",
        0x30066 => "file, folder, or item missing",
        0x30067 => "file missing",
        0x30068 => "folder missing",
        0x30069 => "alias missing",
        0x3006a => "expected a file",
        0x3006b => "expected a folder",
        0x3006c => "expected an alias",
        0x3006d => "alias target missing",
        0x3006e => "read-only volume",
        0x3006f => "volume closed",
        0x30070 => "no space",
        0x30071 => "volume limit",
        0x30072 => "wrong passphrase",
        0x30073 => "name already exists",
        0x30074 => "end of file",
        0x30075 => "file is open",
        0x30076 => "invalid name",
        0x30077 => "access denied by the file system",
        0x30078 => "cross-volume operation",
        0x30079 => "folder too deep or alias loop",
        0x3007a => "volume missing",
        0x40064..=0x4006f => "network errors",
        0x40070 => "Incorrect login and/or password.",
        0x40071 => "The network protocol is unknown.",
        0x40072 => "the peer's protocol is too old",
        0x40073 => "the peer's protocol is more recent",
        0x40074 => "Received data that was invalid according to the protocol.",
        0x40075 => "A limit in the net protocol was exceeded.",
        0x40076 => "can't send fast enough",
        0x40077 => "Banned for flooding.",
        0x50064..=0x50066 => "sound errors",
        _ => return None,
    })
}

/// A message with no payload.
macro_rules! empty_payload {
    ($(#[$doc:meta] $name:ident = $kind:ident;)*) => {
        $(
            #[$doc]
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
            pub struct $name;

            impl $crate::msg::Payload for $name {
                const KIND: $crate::message_type::MessageType = $crate::message_type::MessageType::$kind;

                fn decode(bytes: &[u8]) -> Result<Self, $crate::wire::DecodeError> {
                    $crate::wire::Reader::new(bytes).finish()?;
                    Ok($name)
                }

                fn encode(&self, _w: &mut $crate::wire::Writer) {}
            }
        )*
    };
}

/// A message whose payload is one user record.
macro_rules! user_record_payload {
    ($(#[$doc:meta] $name:ident = $kind:ident;)*) => {
        $(
            #[$doc]
            #[derive(Debug, Clone, PartialEq, Eq)]
            pub struct $name {
                pub user: $crate::msg::common::UserRecord,
            }

            impl $crate::msg::Payload for $name {
                const KIND: $crate::message_type::MessageType = $crate::message_type::MessageType::$kind;

                fn decode(bytes: &[u8]) -> Result<Self, $crate::wire::DecodeError> {
                    let mut r = $crate::wire::Reader::new(bytes);
                    let user = $crate::msg::common::UserRecord::decode(&mut r)?;
                    r.finish()?;
                    Ok($name { user })
                }

                fn encode(&self, w: &mut $crate::wire::Writer) {
                    self.user.encode(w);
                }
            }
        )*
    };
}

pub(crate) use {empty_payload, user_record_payload};

macro_rules! messages {
    ($($variant:ident($ty:path);)*) => {
        /// Every typed payload, or the bytes of one this crate does not model.
        #[derive(Debug, Clone, PartialEq)]
        pub enum Message {
            $($variant($ty),)*
            Unknown { kind: MessageType, bytes: Vec<u8> },
        }

        impl Message {
            /// Decodes `bytes` as the payload of `kind`. An unknown kind gives
            /// [`Message::Unknown`]; a known kind whose bytes do not fit is an error.
            pub fn decode(kind: MessageType, bytes: &[u8]) -> Result<Message, DecodeError> {
                Ok(match kind {
                    $(<$ty as Payload>::KIND => Message::$variant(<$ty as Payload>::decode(bytes)?),)*
                    _ => Message::Unknown { kind, bytes: bytes.to_vec() },
                })
            }

            pub fn kind(&self) -> MessageType {
                match self {
                    $(Message::$variant(_) => <$ty as Payload>::KIND,)*
                    Message::Unknown { kind, .. } => *kind,
                }
            }

            pub fn encode(&self) -> Vec<u8> {
                match self {
                    $(Message::$variant(p) => p.to_bytes(),)*
                    Message::Unknown { bytes, .. } => bytes.clone(),
                }
            }

            /// Whether this crate has a type for `kind`.
            pub fn is_modeled(kind: MessageType) -> bool {
                matches!(kind, $(<$ty as Payload>::KIND)|*)
            }
        }
    };
}

messages! {
    Status(Status);
    SetIdentity(session::SetIdentity);
    UserList(session::UserList);
    UserChanged(session::UserChanged);
    UserJoined(session::UserJoined);
    UserDeparted(session::UserDeparted);
    ServerInfo(session::ServerInfo);
    ServerIcon(session::ServerIcon);
    ServerGreeting(session::ServerGreeting);
    Ping(session::Ping);
    Pong(session::Pong);
    SetMiniUserIcon(session::SetMiniUserIcon);
    MiniUserIconChanged(session::MiniUserIconChanged);
    RequestGreeting(session::RequestGreeting);
    GetExtraServerInfo(session::GetExtraServerInfo);
    ExtraServerInfo(session::ExtraServerInfo);
    GetMyAddress(session::GetMyAddress);
    YourAddress(session::YourAddress);
    Skpk(session::Skpk);
    LicenseReport(session::LicenseReport);
    CreatePrivateChat(chat::CreatePrivateChat);
    DeleteChat(chat::DeleteChat);
    JoinChat(chat::JoinChat);
    ChatWindowInfo(chat::ChatWindowInfo);
    LeaveChat(chat::LeaveChat);
    ChatUserJoined(chat::ChatUserJoined);
    ChatUserDeparted(chat::ChatUserDeparted);
    SendChatText(chat::SendChatText);
    ReceiveChatText(chat::ReceiveChatText);
    SetChatTopic(chat::SetChatTopic);
    ChatTopicChanged(chat::ChatTopicChanged);
    ChatInvite(chat::ChatInvite);
    ChatInvitation(chat::ChatInvitation);
    MustJoinChat(chat::MustJoinChat);
    GetChatList(chat::GetChatList);
    ChatList(chat::ChatList);
    CreatePrivateChatReply(chat::CreatePrivateChatReply);
    CreateChat(chat::CreateChat);
    GetChat(chat::GetChat);
    ChatInfo(chat::ChatInfo);
    SetChat(chat::SetChat);
    ChatDeleted(chat::ChatDeleted);
    GetChatDccAddresses(chat::GetChatDccAddresses);
    ChatDccAddresses(chat::ChatDccAddresses);
    SendPrivateMessage(chat::SendPrivateMessage);
    IncomingPrivateMessage(chat::IncomingPrivateMessage);
    CreateNewsgroup(news::CreateNewsgroup);
    DeleteNewsgroup(news::DeleteNewsgroup);
    GetNewsgroup(news::GetNewsgroup);
    NewsgroupInfo(news::NewsgroupInfoReply);
    SetNewsgroup(news::SetNewsgroup);
    GetNewsgroupList(news::GetNewsgroupList);
    NewsgroupList(news::NewsgroupList);
    GetNewsMessages(news::GetNewsMessages);
    NewsMessages(news::NewsMessages);
    GetNewsMessage(news::GetNewsMessage);
    NewsMessage(news::NewsMessageReply);
    PostNewsMessage(news::PostNewsMessage);
    DeleteNewsMessage(news::DeleteNewsMessage);
    NewsMessagePosted(news::NewsMessagePosted);
    NewsMessageDeleted(news::NewsMessageDeleted);
    GetFileList(files::GetFileList);
    FileList(files::FileList);
    CreateFolder(files::CreateFolder);
    CreateAlias(files::CreateAlias);
    DeleteFileFolder(files::DeleteFileFolder);
    MoveToTrash(files::MoveToTrash);
    RenameFileFolder(files::RenameFileFolder);
    MoveFileFolder(files::MoveFileFolder);
    CopyFileFolder(files::CopyFileFolder);
    GetFileFolderInfo(files::GetFileFolderInfo);
    FileInfo(files::FileInfo);
    SetFileInfo(files::SetFileInfo);
    DownloadFile(files::DownloadFile);
    DownloadInfo(files::DownloadInfo);
    DownloadFileData(files::DownloadFileData);
    UploadFile(files::UploadFile);
    UploadReply(files::UploadReply);
    UploadFileData(files::UploadFileData);
    UploadFileComplete(files::UploadFileComplete);
    NotifySpeedLimit(files::NotifySpeedLimit);
    LaunchFile(files::LaunchFile);
    EmptyTrash(files::EmptyTrash);
    VolumeInfo(files::VolumeInfo);
    SetVolumeInfo(files::SetVolumeInfo);
    GenerateCatalog(files::GenerateCatalog);
    GenerateCatalogStatus(files::GenerateCatalogStatus);
    FileSearch(files::FileSearch);
    FileSearchReply(files::FileSearchReply);
    GetTrackerGroupList(tracker::GetTrackerGroupList);
    TrackerGroupList(tracker::TrackerGroupList);
    GetServerList(tracker::GetServerList);
    TrackerServerList(tracker::TrackerServerList);
    GetTrackerSettings(tracker::GetTrackerSettings);
    TrackerSettingsInfo(tracker::TrackerSettingsInfo);
    SetTrackerSettings(tracker::SetTrackerSettings);
    SetTrackerSettingsDone(tracker::SetTrackerSettingsDone);
    GetTrackerStatistics(tracker::GetTrackerStatistics);
    TrackerStats(tracker::TrackerStats);
    GetAccountClass(accounts::GetAccountClass);
    AccountClassInfo(accounts::AccountClassInfo);
    SetAccountClass(accounts::SetAccountClass);
    GetAccountClassList(accounts::GetAccountClassList);
    AccountClassList(accounts::AccountClassList);
    CreateAccount(accounts::CreateAccount);
    DeleteAccount(accounts::DeleteAccount);
    GetAccount(accounts::GetAccount);
    AccountInfo(accounts::AccountInfo);
    SetAccount(accounts::SetAccount);
    GetAccountList(accounts::GetAccountList);
    AccountList(accounts::AccountList);
    AccountChanged(accounts::AccountChanged);
    AccountDeleted(accounts::AccountDeleted);
    ChangeOwnPassword(accounts::ChangeOwnPassword);
    ChangeOwnPasswordDone(accounts::ChangeOwnPasswordDone);
    OpenConnectionMonitor(admin::OpenConnectionMonitor);
    CloseConnectionMonitor(admin::CloseConnectionMonitor);
    GetConnectionMonitorInfo(admin::GetConnectionMonitorInfo);
    ConnectionMonitorInfo(admin::ConnectionMonitorInfo);
    DisconnectUser(admin::DisconnectUser);
    DisconnectMessage(admin::DisconnectMessage);
    GetUserInfo(admin::GetUserInfo);
    UserInfo(admin::UserInfo);
    SendBroadcast(admin::SendBroadcast);
    RcvBroadcast(admin::RcvBroadcast);
    GetServerSettings(admin::GetServerSettings);
    ServerSettingsInfo(admin::ServerSettingsInfo);
    SetServerSettings(admin::SetServerSettings);
    SetServerSettingsDone(admin::SetServerSettingsDone);
    ShutdownServer(admin::ShutdownServer);
    UpgradeServerSoftware(admin::UpgradeServerSoftware);
    ServerHistoryJoin(admin::ServerHistoryJoin);
    HistoryList(admin::HistoryList);
    ServerHistoryLeave(admin::ServerHistoryLeave);
    HistoryAdded(admin::HistoryAdded);
    OpenProcessMonitor(misc::OpenProcessMonitor);
    CloseProcessMonitor(misc::CloseProcessMonitor);
    GetProcessList(misc::GetProcessList);
    ProcessList(misc::ProcessList);
    ExitTerminateProcess(misc::ExitTerminateProcess);
    OpenViewDisplay(misc::OpenViewDisplay);
    CloseViewDisplay(misc::CloseViewDisplay);
    ViewDisplayParams(misc::ViewDisplayParams);
    ViewDisplayUpdate(misc::ViewDisplayUpdate);
    ViewDisplayMouse(misc::ViewDisplayMouse);
    ViewDisplayKeyboard(misc::ViewDisplayKeyboard);
    StartVoice(misc::StartVoice);
    StartVoiceReply(misc::StartVoiceReply);
    VoiceFormat(misc::VoiceFormat);
    VoiceData(misc::VoiceData);
    CreateFolderAccess(misc::CreateFolderAccess);
    DeleteFolderAccess(misc::DeleteFolderAccess);
    FolderAccessDeleted(misc::FolderAccessDeleted);
    GetFolderAccess(misc::GetFolderAccess);
    FolderAccessInfo(misc::FolderAccessInfo);
    SetFolderAccess(misc::SetFolderAccess);
    GetFolderAccessList(misc::GetFolderAccessList);
    FolderAccessList(misc::FolderAccessList);
    OpenWindow(gconsole::OpenWindow);
    Draw(gconsole::Draw);
    Key(gconsole::Key);
    Mouse(gconsole::Mouse);
}
