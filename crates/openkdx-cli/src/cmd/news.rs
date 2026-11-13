use anyhow::anyhow;
use openkdx_client::session::Session;
use openkdx_proto::msg::news::NewsgroupInfo;
use serde_json::json;

use super::Ctx;
use crate::args::NewsCmd;
use crate::out::{lossy, time_string};

async fn find_group(session: &Session, spec: &str) -> anyhow::Result<u32> {
    if let Ok(id) = spec.parse::<u32>() {
        return Ok(id);
    }
    session
        .newsgroups()
        .await?
        .iter()
        .find(|g| g.name.eq_ignore_ascii_case(spec.as_bytes()))
        .map(|g| g.group_id)
        .ok_or_else(|| anyhow!("no newsgroup named {spec}"))
}

pub async fn news(ctx: &mut Ctx, cmd: NewsCmd) -> anyhow::Result<()> {
    let out = ctx.out;
    let session = ctx.session().await?.clone();
    match cmd {
        NewsCmd::Groups => {
            let rows: Vec<Vec<serde_json::Value>> = session
                .newsgroups()
                .await?
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
            let id = find_group(&session, &group).await?;
            let list = session.news_messages(id).await?;
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
            let id = find_group(&session, &group).await?;
            let m = session.news_message(id, message).await?;
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
            let id = find_group(&session, &group).await?;
            let body = match (body, body_file) {
                (Some(b), _) => b.into_bytes(),
                (None, Some(f)) => std::fs::read(f)?,
                (None, None) => Vec::new(),
            };
            let message_id = session.post_news(id, subject.as_bytes(), &body).await?;
            out.record(
                "news-posted",
                &[("group_id", json!(id)), ("message_id", json!(message_id))],
            );
        }
        NewsCmd::Delete { group, message } => {
            let id = find_group(&session, &group).await?;
            session.delete_news(id, message).await?;
        }
        NewsCmd::SetGroupSize { group, size_kib } => {
            let id = find_group(&session, &group).await?;
            let mut info = session.newsgroup(id).await?;
            info.size_limit_kib = size_kib;
            let changed = session.set_newsgroup(info).await?;
            out.record(
                "newsgroup",
                &[
                    ("group_id", json!(changed.group_id)),
                    ("size_limit_kib", json!(changed.size_limit_kib)),
                ],
            );
        }
        NewsCmd::CreateGroup {
            name,
            description,
            size_kib,
        } => {
            let created = session
                .create_newsgroup(NewsgroupInfo {
                    read_classes: u32::MAX,
                    post_classes: u32::MAX,
                    size_limit_kib: size_kib,
                    name: name.clone().into_bytes(),
                    description: description.into_bytes(),
                    ..NewsgroupInfo::default()
                })
                .await?;
            let group_id = match created {
                Some(g) => g.group_id,
                None => find_group(&session, &name).await?,
            };
            out.record(
                "newsgroup",
                &[("group_id", json!(group_id)), ("name", json!(name))],
            );
        }
        NewsCmd::DeleteGroup { group } => {
            let id = find_group(&session, &group).await?;
            let reply = session.delete_newsgroup(id).await?;
            out.note(format!("server answered {}", reply.kind()));
        }
    }
    Ok(())
}
