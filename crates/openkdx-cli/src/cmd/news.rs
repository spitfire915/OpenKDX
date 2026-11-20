use anyhow::anyhow;
use kdx_client::Client;
use kdx_proto::msg::news::NewsgroupInfo;
use serde_json::json;

use super::{Ctx, within};
use crate::args::NewsCmd;
use crate::out::{lossy, time_string};

async fn find_group(client: &mut Client, spec: &str) -> anyhow::Result<u32> {
    if let Ok(id) = spec.parse::<u32>() {
        return Ok(id);
    }
    client
        .newsgroups()
        .await?
        .iter()
        .find(|g| g.name.eq_ignore_ascii_case(spec.as_bytes()))
        .map(|g| g.group_id)
        .ok_or_else(|| anyhow!("no newsgroup named {spec}"))
}

pub async fn news(ctx: &mut Ctx, cmd: NewsCmd) -> anyhow::Result<()> {
    let out = ctx.out;
    let timeout = ctx.timeout();
    let client = ctx.client().await?;
    match cmd {
        NewsCmd::Groups => {
            let groups = within(timeout, client.newsgroups()).await?;
            let rows: Vec<Vec<serde_json::Value>> = groups
                .iter()
                .map(|g| {
                    vec![
                        json!(g.group_id),
                        json!(lossy(&g.name)),
                        json!(g.message_count),
                        json!(time_string(g.time)),
                        json!(lossy(&g.description)),
                    ]
                })
                .collect();
            out.table(
                "newsgroup",
                &["group_id", "name", "messages", "last_post", "description"],
                &rows,
            );
        }
        NewsCmd::List { group } => {
            let id = find_group(client, &group).await?;
            let list = within(timeout, client.news(id)).await?;
            let rows: Vec<Vec<serde_json::Value>> = list
                .messages
                .iter()
                .map(|m| {
                    vec![
                        json!(m.message_id),
                        json!(time_string(m.time)),
                        json!(lossy(&m.poster)),
                        json!(lossy(&m.subject)),
                    ]
                })
                .collect();
            out.table("news", &["message_id", "time", "poster", "subject"], &rows);
        }
        NewsCmd::Read { group, message } => {
            let id = find_group(client, &group).await?;
            let m = within(timeout, client.news_message(id, message)).await?;
            out.record(
                "news-message",
                &[
                    ("group_id", json!(m.group_id)),
                    ("message_id", json!(m.message_id)),
                    ("time", json!(time_string(m.time))),
                    ("poster", json!(lossy(&m.poster))),
                    ("login", json!(lossy(&m.login))),
                    ("subject", json!(lossy(&m.subject))),
                    ("body", json!(lossy(&m.body).replace('\r', "\n"))),
                ],
            );
        }
        NewsCmd::Post {
            group,
            subject,
            body,
            body_file,
        } => {
            let id = find_group(client, &group).await?;
            let body = match (body, body_file) {
                (Some(b), _) => b.into_bytes(),
                (None, Some(f)) => std::fs::read(f)?,
                (None, None) => Vec::new(),
            };
            let message_id =
                within(timeout, client.post_news_id(id, subject.as_bytes(), &body)).await?;
            out.record(
                "news-posted",
                &[("group_id", json!(id)), ("message_id", json!(message_id))],
            );
        }
        NewsCmd::Delete { group, message } => {
            let id = find_group(client, &group).await?;
            within(timeout, client.delete_news(id, message)).await?;
        }
        NewsCmd::SetGroupSize { group, size_kib } => {
            let id = find_group(client, &group).await?;
            let mut info = within(timeout, client.newsgroup(id)).await?;
            info.size_limit_kib = size_kib;
            let changed = within(timeout, client.set_newsgroup(&info)).await?;
            let size = changed.map_or(size_kib, |g| g.size_limit_kib);
            out.record(
                "newsgroup",
                &[("group_id", json!(id)), ("size_limit_kib", json!(size))],
            );
        }
        NewsCmd::CreateGroup {
            name,
            description,
            size_kib,
        } => {
            let group = NewsgroupInfo {
                read_classes: u32::MAX,
                post_classes: u32::MAX,
                size_limit_kib: size_kib,
                name: name.clone().into_bytes(),
                description: description.into_bytes(),
                ..NewsgroupInfo::default()
            };
            let created = within(timeout, client.create_newsgroup(&group)).await?;
            let group_id = match created {
                Some(g) => g.group_id,
                None => find_group(client, &name).await?,
            };
            out.record(
                "newsgroup",
                &[("group_id", json!(group_id)), ("name", json!(name))],
            );
        }
        NewsCmd::DeleteGroup { group } => {
            let id = find_group(client, &group).await?;
            within(timeout, client.delete_newsgroup(id)).await?;
            out.note(format!("deleted newsgroup {id}"));
        }
    }
    Ok(())
}
