//! News: messages 600–614.

use openkdx_proto::Message;
use openkdx_proto::msg::news::{
    CreateNewsgroup, DeleteNewsMessage, DeleteNewsgroup, GetNewsMessage, GetNewsMessages,
    GetNewsgroup, GetNewsgroupList, NewsMessage, NewsMessageReply, NewsMessages, NewsgroupEntry,
    NewsgroupInfo, NewsgroupInfoReply, NewsgroupList, PostNewsMessage, SetNewsgroup,
};

use crate::Result;
use crate::session::Session;

/// News bodies end lines with CR on the wire.
pub fn to_cr_lines(text: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(text.len());
    let mut i = 0;
    while i < text.len() {
        match text[i] {
            b'\r' if text.get(i + 1) == Some(&b'\n') => {
                out.push(b'\r');
                i += 1;
            }
            b'\n' => out.push(b'\r'),
            b => out.push(b),
        }
        i += 1;
    }
    out
}

impl Session {
    /// 605 → 606.
    pub async fn newsgroups(&self) -> Result<Vec<NewsgroupEntry>> {
        let list: NewsgroupList = self.connection().request_as(&GetNewsgroupList).await?;
        Ok(list.groups)
    }

    /// 602 → 603.
    pub async fn newsgroup(&self, group_id: u32) -> Result<NewsgroupInfo> {
        let reply: NewsgroupInfoReply = self
            .connection()
            .request_as(&GetNewsgroup { group_id })
            .await?;
        Ok(reply.group)
    }

    /// 607 → 608.
    pub async fn news_messages(&self, group_id: u32) -> Result<NewsMessages> {
        self.connection()
            .request_as(&GetNewsMessages { group_id })
            .await
    }

    /// 609 → 610.
    pub async fn news_message(&self, group_id: u32, message_id: u32) -> Result<NewsMessage> {
        let reply: NewsMessageReply = self
            .connection()
            .request_as(&GetNewsMessage {
                group_id,
                message_id,
            })
            .await?;
        Ok(reply.message)
    }

    /// 611; the confirmation is a 613 with its own txid (`[O news-basic]`).
    /// Waits for the 613 of this group and returns the new message ID.
    pub async fn post_news(&self, group_id: u32, subject: &[u8], body: &[u8]) -> Result<u32> {
        let mut events = self.events();
        self.connection()
            .send(&PostNewsMessage {
                group_id,
                ignored_04: [0; 8],
                ignored_0f: 0,
                subject: subject.to_vec(),
                body: to_cr_lines(body),
                rest: Vec::new(),
            })
            .await?;
        let posted = self
            .wait_for(&mut events, |m| match m {
                Message::NewsMessagePosted(p) if p.group_id == group_id => Some(p.message_id),
                _ => None,
            })
            .await?;
        Ok(posted)
    }

    /// 612; the confirmation is a 614 with its own txid.
    pub async fn delete_news(&self, group_id: u32, message_id: u32) -> Result<()> {
        let mut events = self.events();
        self.connection()
            .send(&DeleteNewsMessage {
                group_id,
                message_id,
            })
            .await?;
        self.wait_for(&mut events, |m| match m {
            Message::NewsMessageDeleted(d) if d.group_id == group_id => Some(()),
            _ => None,
        })
        .await
    }

    /// 600 → 603.
    /// 600 → 603, or a success Status (Server 1.620 answers with a Status
    /// and lists the new group in the next 606; `[O lab]`). `None` then.
    pub async fn create_newsgroup(&self, group: NewsgroupInfo) -> Result<Option<NewsgroupInfo>> {
        match self
            .connection()
            .request_ok(&CreateNewsgroup { group })
            .await?
        {
            Message::NewsgroupInfo(NewsgroupInfoReply { group }) => Ok(Some(group)),
            _ => Ok(None),
        }
    }

    /// 604 → 603, or a success Status (Server 1.620; `[O lab]`), `None` then.
    pub async fn set_newsgroup(&self, group: NewsgroupInfo) -> Result<Option<NewsgroupInfo>> {
        match self
            .connection()
            .request_ok(&SetNewsgroup { group })
            .await?
        {
            Message::NewsgroupInfo(NewsgroupInfoReply { group }) => Ok(Some(group)),
            _ => Ok(None),
        }
    }

    /// 601; whatever the server answers.
    pub async fn delete_newsgroup(&self, group_id: u32) -> Result<Message> {
        self.connection()
            .request(&DeleteNewsgroup { group_id })
            .await
    }
}
