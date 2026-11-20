use std::time::Duration;

use kdx_client::{Notice, set_server_name};
use kdx_proto::msg::admin::{HistoryRecord, MonitoredConnection};
use serde_json::{Value, json};

use super::{Ctx, find_user, path_string, within};
use crate::args::ServerCmd;
use crate::cmd::session::watch;
use crate::out::{Out, lossy, time_string};

fn history_row(r: &HistoryRecord) -> Vec<Value> {
    vec![
        json!(time_string(r.time)),
        json!(r.kind.title().unwrap_or("?")),
        json!(r.user_id),
        json!(lossy(&r.nick)),
        json!(lossy(&r.login)),
        json!(lossy(&r.address)),
        json!(lossy(&r.detail)),
        json!(path_string(&r.path)),
    ]
}

const HISTORY_COLS: [&str; 8] = [
    "time", "event", "user_id", "nick", "login", "address", "detail", "path",
];

fn monitor_row(c: &MonitoredConnection) -> Vec<Value> {
    vec![
        json!(c.user_id),
        json!(c.transfer_id),
        json!(lossy(&c.nick)),
        json!(lossy(&c.login)),
        json!(lossy(&c.address)),
        json!(time_string(c.login_time)),
        json!(c.bytes_done),
        json!(c.bytes_total),
        json!(c.rate),
        json!(path_string(&c.path)),
    ]
}

const MONITOR_COLS: [&str; 10] = [
    "user_id",
    "transfer_id",
    "nick",
    "login",
    "address",
    "login_time",
    "bytes_done",
    "bytes_total",
    "rate",
    "path",
];

pub async fn server(ctx: &mut Ctx, cmd: ServerCmd) -> anyhow::Result<()> {
    let out: Out = ctx.out;
    let timeout = ctx.timeout();
    let client = ctx.client().await?;
    match cmd {
        ServerCmd::Settings => {
            let s = within(timeout, client.server_settings()).await?;
            let main = &s.main;
            let sections: Vec<u32> = s.sections().iter().map(|sec| sec.len).collect();
            out.record(
                "server-settings",
                &[
                    ("version", json!(s.version)),
                    ("name", json!(lossy(&main.name))),
                    ("description", json!(lossy(&main.description))),
                    ("address", json!(lossy(&main.address))),
                    ("port", json!(main.port)),
                    ("alternate_port", json!(main.alternate_port)),
                    ("file_transfer_port", json!(main.file_transfer_port)),
                    ("greeting_enabled", json!(main.greeting_enabled())),
                    (
                        "greeting",
                        json!(s.greeting.as_ref().map(|g| lossy(&g.text))),
                    ),
                    (
                        "register_with_trackers",
                        json!(main.register_with_trackers()),
                    ),
                    (
                        "total_outgoing_speed_limit",
                        json!(main.total_outgoing_speed_limit),
                    ),
                    ("history_kinds", json!(main.history_kinds)),
                    ("history_max_file_size", json!(main.history_max_file_size)),
                    (
                        "trackers",
                        json!(
                            s.trackers
                                .iter()
                                .map(|t| lossy(&t.address))
                                .collect::<Vec<_>>()
                        ),
                    ),
                    (
                        "allow_deny",
                        json!(
                            s.allow_deny
                                .iter()
                                .map(|a| format!(
                                    "{} {}",
                                    if a.allow { "allow" } else { "deny" },
                                    lossy(&a.address)
                                ))
                                .collect::<Vec<_>>()
                        ),
                    ),
                    ("icon_bytes", json!(s.icon.as_ref().map_or(0, |i| i.len()))),
                    ("section_lengths", json!(sections)),
                ],
            );
        }
        ServerCmd::SetName { name } => {
            let mut s = within(timeout, client.server_settings()).await?;
            set_server_name(&mut s, name.as_bytes())?;
            within(timeout, client.set_server_settings(&s)).await?;
        }
        ServerCmd::SetHistory { kinds, max_kib } => {
            let kinds = kinds
                .strip_prefix("0x")
                .map_or_else(|| kinds.parse::<u32>(), |hex| u32::from_str_radix(hex, 16))?;
            let mut s = within(timeout, client.server_settings()).await?;
            s.main.history_kinds = kinds;
            if let Some(kib) = max_kib {
                s.main.history_max_file_size = kib * 1024;
            }
            within(timeout, client.set_server_settings(&s)).await?;
        }
        ServerCmd::Broadcast { text } => {
            within(timeout, client.broadcast(text.join(" ").as_bytes())).await?;
        }
        ServerCmd::Disconnect {
            user,
            ban,
            message,
            quiet,
        } => {
            let id = find_user(client, &user)?;
            within(
                timeout,
                client.disconnect_user(id, ban, message.as_bytes(), quiet),
            )
            .await?;
        }
        ServerCmd::History { follow } => {
            let list = within(timeout, client.history_join()).await?;
            let rows: Vec<Vec<Value>> = list.records.iter().map(history_row).collect();
            out.table("history", &HISTORY_COLS, &rows);
            if let Some(secs) = follow {
                watch(client, out, Duration::from_secs(secs), |_| false).await?;
            }
            client.history_leave().await?;
        }
        ServerCmd::Monitor { r#for, every } => {
            client.monitor_open().await?;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(r#for.unwrap_or(0));
            loop {
                let info = within(timeout, client.monitor_info()).await?;
                let rows: Vec<Vec<Value>> = info.iter().map(monitor_row).collect();
                out.table("connection", &MONITOR_COLS, &rows);
                if tokio::time::Instant::now() >= deadline {
                    break;
                }
                out.note("");
                tokio::time::sleep(Duration::from_secs(every.max(1))).await;
            }
            client.monitor_close().await?;
        }
        ServerCmd::Shutdown { mode, message } => {
            client.shutdown(mode, message.as_bytes()).await?;
            let _ = watch(client, out, Duration::from_secs(5), |n| {
                matches!(n, Notice::Disconnect(_))
            })
            .await;
        }
        ServerCmd::Info => {
            let extra = within(timeout, client.extra_server_info()).await?;
            let address = client.my_address().await.unwrap_or_default();
            out.record(
                "server-extra",
                &[
                    ("start_time", json!(time_string(extra.start_time))),
                    ("now", json!(time_string(extra.now))),
                    ("uptime_seconds", json!(extra.uptime)),
                    ("os", json!(lossy(&extra.os))),
                    ("your_address", json!(lossy(&address))),
                ],
            );
        }
    }
    Ok(())
}
