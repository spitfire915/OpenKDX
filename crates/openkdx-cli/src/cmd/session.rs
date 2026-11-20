use std::io::BufRead;
use std::time::Duration;

use clap::Parser;
use kdx_client::{Client, Notice};
use serde_json::json;

use super::{Ctx, Disconnected, dispatch};
use crate::args::RunLine;
use crate::out::{Out, lossy, version_string};

pub async fn login(ctx: &mut Ctx, stay: Option<u64>) -> anyhow::Result<()> {
    let out = ctx.out;
    let client = ctx.client().await?;
    let info = &client.info;
    out.record(
        "server",
        &[
            ("name", json!(lossy(&info.name))),
            ("description", json!(lossy(&info.description))),
            ("version", json!(version_string(info.version))),
            ("address", json!(lossy(&info.address))),
            ("user_id", json!(info.user_id)),
            ("users", json!(client.users.len())),
            ("transfer_port", json!(info.file_transfer_port)),
            ("can_modify_settings", json!(info.can_modify_settings)),
        ],
    );
    // The greeting usually lands just after 105; give it a moment either way.
    let wait = stay.map_or(Duration::from_millis(200), Duration::from_secs);
    watch(client, out, wait, |_| false).await
}

pub async fn users(ctx: &mut Ctx) -> anyhow::Result<()> {
    let out = ctx.out;
    let client = ctx.client().await?;
    let rows: Vec<_> = client.users.iter().map(Out::user_row).collect();
    out.table("user", &["user_id", "nick", "status", "info"], &rows);
    Ok(())
}

/// Prints notices for `duration`, or until `done` says so. A 151
/// Disconnect Message, or the connection closing, ends it with exit code 3.
pub async fn watch(
    client: &mut Client,
    out: Out,
    duration: Duration,
    mut done: impl FnMut(&Notice) -> bool,
) -> anyhow::Result<()> {
    let deadline = tokio::time::Instant::now() + duration;
    loop {
        let notice = tokio::select! {
            () = tokio::time::sleep_until(deadline) => return Ok(()),
            notice = client.next_notice() => match notice {
                Ok(notice) => notice,
                // The cause (a closed socket, a frame that did not decode)
                // stays in the chain under the typed error.
                Err(e) => return Err(e.context(Disconnected::Gone)),
            },
        };
        apply(client, &notice);
        out.event(&notice, &client.users);
        if let Notice::Disconnect(d) = &notice {
            return Err(Disconnected::By(lossy(&d.message)).into());
        }
        if done(&notice) {
            return Ok(());
        }
    }
}

/// Keeps the user list current, so later nick lookups see who joined.
fn apply(client: &mut Client, notice: &Notice) {
    match notice {
        Notice::UserJoined(u) | Notice::UserChanged(u) => {
            match client.users.iter_mut().find(|x| x.user_id == u.user_id) {
                Some(slot) => *slot = u.clone(),
                None => client.users.push(u.clone()),
            }
        }
        Notice::UserDeparted(d) => client.users.retain(|x| x.user_id != d.user_id),
        _ => {}
    }
}

/// Commands from stdin, one per line, on this context's client.
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
        let Some(words) = split_line(line) else {
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

/// Words of a `run` line: whitespace separates, single or double quotes
/// group, and a backslash is an ordinary character (Windows paths pass
/// through). `None` on an unclosed quote.
pub fn split_line(line: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut in_word = false;
    let mut quote: Option<char> = None;
    for c in line.chars() {
        match quote {
            Some(q) if c == q => quote = None,
            Some(_) => word.push(c),
            None if c == '\'' || c == '"' => {
                quote = Some(c);
                in_word = true;
            }
            None if c.is_whitespace() => {
                if in_word {
                    words.push(std::mem::take(&mut word));
                    in_word = false;
                }
            }
            None => {
                word.push(c);
                in_word = true;
            }
        }
    }
    if quote.is_some() {
        return None;
    }
    if in_word {
        words.push(word);
    }
    Some(words)
}

#[cfg(test)]
mod tests {
    use super::split_line;

    #[test]
    fn split_line_keeps_backslashes_and_groups_quotes() {
        assert_eq!(
            split_line(r#"files put C:\Temp\up.bin "Uploads [UL]" --name 'a b'"#).unwrap(),
            vec![
                "files",
                "put",
                r"C:\Temp\up.bin",
                "Uploads [UL]",
                "--name",
                "a b"
            ]
        );
        assert_eq!(split_line("  users  ").unwrap(), vec!["users"]);
        assert!(split_line("say 1 \"unclosed").is_none());
    }
}
