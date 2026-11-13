use std::io::BufRead;
use std::time::Duration;

use clap::Parser;
use openkdx_client::connection::Event;
use serde_json::json;

use super::{Ctx, dispatch};
use crate::args::RunLine;
use crate::out::{Out, lossy, version_string};

pub async fn login(ctx: &mut Ctx, stay: Option<u64>) -> anyhow::Result<()> {
    let session = ctx.session().await?.clone();
    let out = ctx.out;
    out.record(
        "server",
        &[
            ("name", json!(lossy(&session.server.name))),
            ("description", json!(lossy(&session.server.description))),
            ("version", json!(version_string(session.server.version))),
            ("address", json!(lossy(&session.server.address))),
            ("user_id", json!(session.user_id())),
            ("users", json!(session.users().len())),
            ("transfer_port", json!(session.server.file_transfer_port)),
            (
                "can_modify_settings",
                json!(session.server.can_modify_settings),
            ),
        ],
    );
    if let Some(stay) = stay {
        // The greeting usually lands just after 105; give it a moment.
        tokio::time::sleep(Duration::from_millis(200)).await;
        if let Some(g) = session.greeting() {
            out.record("greeting", &[("text", json!(lossy(&g)))]);
        }
        watch(&session, out, Duration::from_secs(stay), |_| false).await?;
    } else if let Some(g) = session.greeting() {
        out.record("greeting", &[("text", json!(lossy(&g)))]);
    }
    Ok(())
}

pub async fn users(ctx: &mut Ctx) -> anyhow::Result<()> {
    let session = ctx.session().await?;
    let rows: Vec<_> = session.users().iter().map(Out::user_row).collect();
    ctx.out
        .table("user", &["user_id", "nick", "status", "info"], &rows);
    Ok(())
}

/// Print events for `duration`, or until `done` says so. A disconnect ends
/// it with the server's message as the error.
pub async fn watch(
    session: &openkdx_client::session::Session,
    out: Out,
    duration: Duration,
    mut done: impl FnMut(&Event) -> bool,
) -> anyhow::Result<()> {
    let mut events = session.events();
    let deadline = tokio::time::Instant::now() + duration;
    loop {
        let event = match tokio::time::timeout_at(deadline, events.recv()).await {
            Err(_) => return Ok(()),
            Ok(Err(tokio::sync::broadcast::error::RecvError::Lagged(_))) => continue,
            Ok(Err(_)) => return Err(openkdx_client::ClientError::Disconnected.into()),
            Ok(Ok(e)) => e,
        };
        out.event(&event, &session.users());
        if let Event::Disconnected(d) = &event {
            return Err(match &d.message {
                Some(m) => openkdx_client::ClientError::DisconnectedBy(Box::new(m.clone())),
                None => openkdx_client::ClientError::Disconnected,
            }
            .into());
        }
        if done(&event) {
            return Ok(());
        }
    }
}

/// Commands from stdin, one per line, on this context's session.
pub async fn run(ctx: &mut Ctx, keep_going: bool) -> anyhow::Result<()> {
    let stdin = std::io::stdin();
    let lines: Vec<String> = stdin.lock().lines().collect::<Result<_, _>>()?;
    let mut failed = false;
    for line in lines {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        ctx.out.note(format!("> {line}"));
        let Some(words) = shlex::split(line) else {
            anyhow::bail!("unbalanced quotes: {line}");
        };
        let result = match words[0].as_str() {
            "quit" | "exit" => break,
            "sleep" => {
                let secs: f64 = words.get(1).and_then(|s| s.parse().ok()).unwrap_or(1.0);
                tokio::time::sleep(Duration::from_secs_f64(secs)).await;
                Ok(())
            }
            _ => match RunLine::try_parse_from(&words) {
                Ok(RunLine {
                    command: crate::args::Command::Run { .. },
                }) => Err(anyhow::anyhow!("run cannot nest")),
                Ok(parsed) => Box::pin(dispatch(ctx, parsed.command)).await,
                Err(e) => Err(anyhow::anyhow!("{line}: {e}")),
            },
        };
        if let Err(e) = result {
            eprintln!("error: {e:#}");
            failed = true;
            if !keep_going {
                return Err(e);
            }
        }
    }
    if failed {
        anyhow::bail!("some commands failed");
    }
    Ok(())
}
