use std::time::Duration;

use anyhow::anyhow;
use kdx_client::{Client, Notice, PUBLIC_CHAT};
use serde_json::json;

use super::{Ctx, find_user, within};
use crate::args::{ChatCmd, PmCmd};
use crate::cmd::session::watch;
use crate::out::lossy;

/// A chat named by ID or name; `(chat_id, key)`.
async fn find_chat(client: &mut Client, spec: &str) -> anyhow::Result<(u32, u32)> {
    if let Ok(id) = spec.parse::<u32>() {
        if id == PUBLIC_CHAT {
            return Ok((id, 0));
        }
        let list = client.chat_list().await?;
        return Ok(list
            .iter()
            .find(|c| c.chat_id == id)
            .map(|c| (c.chat_id, c.key))
            .unwrap_or((id, 0)));
    }
    let list = client.chat_list().await?;
    list.iter()
        .find(|c| c.name.eq_ignore_ascii_case(spec.as_bytes()))
        .map(|c| (c.chat_id, c.key))
        .ok_or_else(|| anyhow!("no chat named {spec}"))
}

fn joined(words: &[String]) -> Vec<u8> {
    words.join(" ").into_bytes()
}

fn nick_of(client: &Client, id: u32) -> String {
    client
        .users
        .iter()
        .find(|u| u.user_id == id)
        .map(|u| lossy(&u.nick))
        .unwrap_or_else(|| format!("#{id}"))
}

pub async fn chat(ctx: &mut Ctx, cmd: ChatCmd) -> anyhow::Result<()> {
    let out = ctx.out;
    let timeout = ctx.timeout();
    let client = ctx.client().await?;
    match cmd {
        ChatCmd::List => {
            let list = within(timeout, client.chat_list()).await?;
            let rows: Vec<Vec<serde_json::Value>> = list
                .iter()
                .map(|c| {
                    vec![
                        json!(c.chat_id),
                        json!(lossy(&c.name)),
                        json!(c.member_count),
                        json!(c.flags & 1 != 0),
                        json!(lossy(&c.description)),
                    ]
                })
                .collect();
            out.table(
                "chat",
                &["chat_id", "name", "members", "private", "description"],
                &rows,
            );
        }
        ChatCmd::Join { chat } => {
            let (id, key) = find_chat(client, &chat).await?;
            let info = within(timeout, client.join_chat(id, key)).await?;
            let members: Vec<String> = info.members.iter().map(|m| nick_of(client, *m)).collect();
            out.record(
                "chat-window",
                &[
                    ("chat_id", json!(info.chat_id)),
                    ("name", json!(lossy(&info.name))),
                    ("topic", json!(lossy(&info.topic))),
                    ("topic_setter", json!(lossy(&info.topic_setter))),
                    ("members", json!(members)),
                ],
            );
        }
        ChatCmd::Say { chat, text } => {
            let (id, _) = find_chat(client, &chat).await?;
            client.say(id, &joined(&text), false).await?;
        }
        ChatCmd::Me { chat, text } => {
            let (id, _) = find_chat(client, &chat).await?;
            client.say(id, &joined(&text), true).await?;
        }
        ChatCmd::Topic { chat, text } => {
            let (id, _) = find_chat(client, &chat).await?;
            within(timeout, client.set_topic(id, &joined(&text))).await?;
        }
        ChatCmd::Leave { chat } => {
            let (id, _) = find_chat(client, &chat).await?;
            client.leave_chat(id).await?;
        }
        ChatCmd::Watch { chat, r#for } => {
            let (id, key) = find_chat(client, &chat).await?;
            let info = within(timeout, client.join_chat(id, key)).await?;
            out.note(format!(
                "joined {} ({} members), topic: {}",
                lossy(&info.name),
                info.members.len(),
                lossy(&info.topic)
            ));
            watch(client, out, Duration::from_secs(r#for), |_| false).await?;
        }
    }
    Ok(())
}

pub async fn pm(ctx: &mut Ctx, cmd: PmCmd) -> anyhow::Result<()> {
    let out = ctx.out;
    let client = ctx.client().await?;
    match cmd {
        PmCmd::Send { user, text } => {
            let id = find_user(client, &user)?;
            client.send_private_message(id, &joined(&text)).await?;
        }
        PmCmd::Wait { r#for } => {
            watch(client, out, Duration::from_secs(r#for), |n| {
                matches!(n, Notice::PrivateMessage(_))
            })
            .await?;
        }
    }
    Ok(())
}
