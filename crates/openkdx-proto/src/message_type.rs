//! Message types and their names, from the server's name table as the
//! reference spec's `messages/README.md` lists them.

use std::fmt;

/// The `kind` field of a session header.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub struct MessageType(pub u32);

macro_rules! message_types {
    ($($num:literal $name:ident $text:literal;)*) => {
        impl MessageType {
            $(pub const $name: MessageType = MessageType($num);)*

            /// The spec's name for the type, if it is known.
            pub fn name(self) -> Option<&'static str> {
                match self.0 {
                    $($num => Some($text),)*
                    _ => None,
                }
            }

            /// Every known type, in numeric order.
            pub const ALL: &'static [MessageType] = &[$(MessageType($num),)*];
        }
    };
}

message_types! {
    1 STATUS "Status";
    100 SET_IDENTITY "Set Identity";
    101 USER_LIST "User List";
    102 USER_CHANGED "User Changed";
    103 USER_JOINED "User Joined";
    104 USER_DEPARTED "User Departed";
    105 SERVER_INFO "Server Info";
    106 SERVER_ICON "Server Icon";
    107 SERVER_GREETING "Server Greeting";
    108 PING "Ping";
    109 PONG "Pong";
    110 SET_MINI_USER_ICON "Set Mini User Icon";
    111 MINI_USER_ICON_CHANGED "Mini User Icon Changed";
    112 REQUEST_GREETING "(unnamed; requests the greeting)";
    113 GET_EXTRA_SERVER_INFO "Get Extra Server Info";
    114 EXTRA_SERVER_INFO "Extra Server Info";
    115 GET_MY_ADDRESS "Get My Address";
    116 YOUR_ADDRESS "Your Address";
    117 SKPK "SKPK";
    118 LICENSE_REPORT "(unnamed; license report)";
    140 OPEN_CONNECTION_MONITOR "Open Connection Monitor";
    141 CLOSE_CONNECTION_MONITOR "Close Connection Monitor";
    142 GET_CONNECTION_MONITOR_INFO "Get Connection Monitor Info";
    143 CONNECTION_MONITOR_INFO "Connection Monitor Info";
    150 DISCONNECT_USER "Disconnect User";
    151 DISCONNECT_MESSAGE "Disconnect Message";
    152 GET_USER_INFO "Get User Info";
    153 USER_INFO "User Info";
    154 SEND_BROADCAST_MESSAGE "Send Broadcast Message";
    155 RCV_BROADCAST_MESSAGE "Rcv Broadcast Message";
    170 GET_SERVER_SETTINGS "Get Server Settings";
    171 SERVER_SETTINGS_INFO "Server Settings Info";
    172 SET_SERVER_SETTINGS "Set Server Settings";
    173 SET_SERVER_SETTINGS_DONE "Set Server Settings Done";
    174 SHUTDOWN_SERVER "Shutdown Server";
    175 UPGRADE_SERVER_SOFTWARE "Upgrade Server Software";
    180 SERVER_HISTORY_JOIN "Server History Join";
    181 SERVER_HISTORY_JOIN_REPLY "Server History Join (R)";
    182 SERVER_HISTORY_LEAVE "Server History Leave";
    183 SERVER_HISTORY_ADDED "Server History Added";
    184 HISTORY_RECORD_184 "History record kind 184";
    185 HISTORY_RECORD_185 "History record kind 185";
    186 HISTORY_RECORD_186 "History record kind 186";
    187 HISTORY_RECORD_187 "History record kind 187";
    188 HISTORY_RECORD_188 "History record kind 188";
    189 HISTORY_RECORD_189 "History record kind 189";
    190 HISTORY_RECORD_190 "History record kind 190";
    191 HISTORY_RECORD_191 "History record kind 191";
    200 CREATE_PRIVATE_CHAT "Create Private Chat";
    201 DELETE_CHAT "Delete Chat";
    202 JOIN_CHAT "Join Chat";
    203 CHAT_WINDOW_INFO "Chat Window Info";
    204 LEAVE_CHAT "Leave Chat";
    205 CHAT_USER_JOINED "Chat User Joined";
    206 CHAT_USER_DEPARTED "Chat User Departed";
    207 SEND_CHAT_TEXT "Send Chat Text";
    208 RECEIVE_CHAT_TEXT "Receive Chat Text";
    209 SET_CHAT_TOPIC "Set Chat Topic";
    210 CHAT_TOPIC_CHANGED "Chat Topic Changed";
    211 CHAT_INVITE "Chat Invite";
    212 CHAT_INVITATION "Chat Invitation";
    213 MUST_JOIN_CHAT "Must Join Chat";
    214 GET_CHAT_LIST "Get Chat List";
    215 CHAT_LIST "Chat List";
    216 CREATE_PRIVATE_CHAT_REPLY "Create Private Chat (R)";
    217 CREATE_CHAT "Create Chat";
    218 GET_CHAT "Get Chat";
    219 CHAT_INFO "Chat Info";
    220 SET_CHAT "Set Chat";
    221 CHAT_DELETED "Chat Deleted";
    222 GET_CHAT_DCC_ADDRESSES "Get Chat DCC Addresses";
    223 CHAT_DCC_ADDRESSES "Chat DCC Addresses";
    300 SEND_PRIVATE_MESSAGE "Send Private Message";
    301 INCOMING_PRIVATE_MESSAGE "Incoming Private Message";
    400 GET_FILE_LIST "Get File List";
    401 FILE_LIST "File List";
    402 CREATE_FOLDER "Create Folder";
    403 CREATE_ALIAS "Create Alias";
    404 DELETE_FILE "Delete File/Folder";
    405 MOVE_TO_TRASH "Move To Trash";
    406 RENAME_FILE "Rename File/Folder";
    407 MOVE_FILE "Move File/Folder";
    408 COPY_FILE "Copy File/Folder";
    409 GET_FILE_INFO "Get File/Folder Info";
    410 FILE_INFO "File/Folder Info";
    411 SET_FILE_INFO "Set File/Folder Info";
    412 DOWNLOAD_FILE "Download File";
    413 DOWNLOAD_FILE_REPLY "Download File (R)";
    414 DOWNLOAD_FILE_DATA "Download File (D)";
    415 UPLOAD_FILE "Upload File";
    416 UPLOAD_FILE_REPLY "Upload File (R)";
    417 UPLOAD_FILE_DATA "Upload File (D)";
    418 UPLOAD_FILE_COMPLETE "Upload File (C)";
    419 NOTIFY_SPEED_LIMIT "Notify Speed Limit";
    420 LAUNCH_FILE "Launch File";
    421 EMPTY_TRASH "Empty Trash";
    422 VOLUME_INFO "Volume Info";
    423 SET_VOLUME_INFO "Set Volume Info";
    450 GENERATE_CATALOG "Generate Catalog";
    451 GENERATE_CATALOG_STATUS "Generate Catalog (S)";
    452 FILE_SEARCH "File Search";
    453 FILE_SEARCH_REPLY "File Search (R)";
    502 GET_ACCOUNT_CLASS "Get Account Class";
    503 ACCOUNT_CLASS_INFO "Account Class Info";
    504 SET_ACCOUNT_CLASS "Set Account Class";
    505 GET_ACCOUNT_CLASS_LIST "Get Account Class List";
    506 ACCOUNT_CLASS_LIST "Account Class List";
    507 CREATE_ACCOUNT "Create Account";
    508 DELETE_ACCOUNT "Delete Account";
    509 GET_ACCOUNT "Get Account";
    510 ACCOUNT_INFO "Account Info";
    511 SET_ACCOUNT "Set Account";
    512 GET_ACCOUNT_LIST "Get Account List";
    513 ACCOUNT_LIST "Account List";
    514 ACCOUNT_CHANGED "Account Changed";
    515 ACCOUNT_DELETED "Account Deleted";
    550 CHANGE_OWN_PASSWORD "Change Own Password";
    551 CHANGE_OWN_PASSWORD_DONE "Change Own Password Done";
    600 CREATE_NEWSGROUP "Create Newsgroup";
    601 DELETE_NEWSGROUP "Delete Newsgroup";
    602 GET_NEWSGROUP "Get Newsgroup";
    603 NEWSGROUP_INFO "Newsgroup Info";
    604 SET_NEWSGROUP "Set Newsgroup";
    605 GET_NEWSGROUP_LIST "Get Newsgroup List";
    606 NEWSGROUP_LIST "Newsgroup List";
    607 GET_NEWS_MESSAGES "Get News Messages";
    608 NEWS_MESSAGES "News Messages";
    609 GET_NEWS_MESSAGE "Get News Message";
    610 NEWS_MESSAGE "News Message";
    611 POST_NEWS_MESSAGE "Post News Message";
    612 DELETE_NEWS_MESSAGE "Delete News Message";
    613 NEWS_MESSAGE_POSTED "News Message Posted";
    614 NEWS_MESSAGE_DELETED "News Message Deleted";
    700 GET_TRACKER_GROUP_LIST "Get Tracker Group List";
    701 TRACKER_GROUP_LIST "Tracker Group List";
    702 GET_TRACKER_SERVER_LIST "Get Tracker Server List";
    703 TRACKER_SERVER_LIST "Tracker Server List";
    710 GET_TRACKER_SETTINGS "Get TrackerServ Settings";
    711 TRACKER_SETTINGS_INFO "TrackerServ Settings Info";
    712 SET_TRACKER_SETTINGS "Set TrackerServ Settings";
    713 SET_TRACKER_SETTINGS_DONE "Set TrackerServ Settings Done";
    720 GET_TRACKER_STATISTICS "Get TrackerServ Statistics";
    721 TRACKER_STATISTICS "TrackerServ Statistics";
    800 OPEN_PROCESS_MONITOR "Open Process Monitor";
    801 CLOSE_PROCESS_MONITOR "Close Process Monitor";
    802 GET_PROCESS_LIST "Get Process List";
    803 PROCESS_LIST "Process List";
    804 TERMINATE_PROCESS "Exit/Terminate Process";
    900 OPEN_VIEW_DISPLAY "Open View Display";
    901 CLOSE_VIEW_DISPLAY "Close View Display";
    902 VIEW_DISPLAY_PARAMS "View Display Params";
    903 VIEW_DISPLAY_UPDATE "View Display Update";
    904 VIEW_DISPLAY_MOUSE "View Display Mouse";
    905 VIEW_DISPLAY_KEYBOARD "View Display Keyboard";
    1000 START_VOICE "Start Voice";
    1001 START_VOICE_REPLY "Start Voice (R)";
    1002 SET_VOICE_FORMAT "Set Voice Format";
    1003 VOICE_DATA "Voice Data";
    1100 CREATE_FOLDER_ACCESS "Create Folder Access";
    1101 DELETE_FOLDER_ACCESS "Delete Folder Access";
    1102 FOLDER_ACCESS_DELETED "Folder Access Deleted";
    1103 GET_FOLDER_ACCESS "Get Folder Access";
    1104 FOLDER_ACCESS_INFO "Folder Access Info";
    1105 SET_FOLDER_ACCESS "Set Folder Access";
    1106 GET_FOLDER_ACCESS_LIST "Get Folder Access List";
    1107 FOLDER_ACCESS_LIST "Folder Access List";
    4000 GCONSOLE_OPEN_WINDOW "Open Window";
    4001 GCONSOLE_DRAW "Draw";
    4200 GCONSOLE_KEY "Key";
    4201 GCONSOLE_MOUSE "Mouse";
}

impl MessageType {
    pub fn is_known(self) -> bool {
        self.name().is_some()
    }
}

impl fmt::Display for MessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.0, self.name().unwrap_or("(unknown)"))
    }
}
