//! Private messages: 300 and 301.

use openkdx_proto::msg::chat::SendPrivateMessage;

use crate::session::Session;
use crate::{ClientError, Result};

impl Session {
    /// 300; no reply, the recipient gets 301.
    pub async fn send_pm(&self, user_id: u32, text: &[u8]) -> Result<()> {
        if text.is_empty() {
            return Err(ClientError::Protocol("empty private message"));
        }
        self.connection()
            .send(&SendPrivateMessage {
                user_id,
                ignored: [0; 12],
                text: text.to_vec(),
            })
            .await
            .map(|_| ())
    }
}
