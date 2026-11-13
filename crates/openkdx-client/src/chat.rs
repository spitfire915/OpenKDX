//! Chat rooms: messages 200–223.

use openkdx_proto::msg::chat::{
    ChatInfo, ChatInvite, ChatList, ChatListEntry, ChatWindowInfo, CreateChat, GetChatList,
    JoinChat, LeaveChat, SendChatText, SetChatTopic,
};
use openkdx_proto::msg::common::ChatRecord;

use crate::session::Session;
use crate::{ClientError, Result};

/// The chat every server has.
pub const PUBLIC_CHAT: u32 = 1;

impl Session {
    /// 214 → 215.
    pub async fn chat_list(&self) -> Result<Vec<ChatListEntry>> {
        let list: ChatList = self.connection().request_as(&GetChatList).await?;
        Ok(list.chats)
    }

    /// 202 → 203. `key` comes from the chat list entry (0 for public chats).
    pub async fn join_chat(&self, chat_id: u32, key: u32) -> Result<ChatWindowInfo> {
        self.connection()
            .request_as(&JoinChat { chat_id, key })
            .await
    }

    /// 204; no reply.
    pub async fn leave_chat(&self, chat_id: u32) -> Result<()> {
        self.connection()
            .send(&LeaveChat { chat_id })
            .await
            .map(|_| ())
    }

    /// 207; no reply, the server echoes it as 208 to every member. `action`
    /// sets the `/me` flag (`docs/protocol/README.md`).
    pub async fn say(&self, chat_id: u32, text: &[u8], action: bool) -> Result<()> {
        if text.len() > SendChatText::MAX_TEXT {
            return Err(ClientError::Protocol("chat text longer than 0x1000 bytes"));
        }
        self.connection()
            .send(&SendChatText {
                chat_id,
                user_id: 0,
                flags: if action { SendChatText::FLAG_ACTION } else { 0 },
                ignored_0a: [0; 2],
                text: text.to_vec(),
            })
            .await
            .map(|_| ())
    }

    /// 209; no reply, the server sends 210 to every member.
    pub async fn set_topic(&self, chat_id: u32, topic: &[u8]) -> Result<()> {
        self.connection()
            .send(&SetChatTopic {
                chat_id,
                ignored_04: [0; 6],
                ignored_0b: 0,
                topic: topic.to_vec(),
                rest: Vec::new(),
            })
            .await
            .map(|_| ())
    }

    /// 217 → 219.
    pub async fn create_chat(&self, chat: ChatRecord) -> Result<ChatRecord> {
        let info: ChatInfo = self.connection().request_as(&CreateChat { chat }).await?;
        Ok(info.chat)
    }

    /// 211; no reply.
    pub async fn invite(&self, chat_id: u32, user_ids: &[u32], message: &[u8]) -> Result<()> {
        if user_ids.len() > 20 {
            return Err(ClientError::Protocol("more than 20 invitees"));
        }
        self.connection()
            .send(&ChatInvite {
                chat_id,
                ignored_04: [0; 4],
                ignored_0a: [0; 5],
                user_ids: user_ids.to_vec(),
                message: message.to_vec(),
            })
            .await
            .map(|_| ())
    }
}
