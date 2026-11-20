use kdx_client::{Login, Tracker};
use serde_json::json;

use super::{Ctx, resolve, within};
use crate::args::TrackerCmd;
use crate::out::{lossy, time_string};

pub async fn tracker(ctx: &mut Ctx, spec: &str, cmd: TrackerCmd) -> anyhow::Result<()> {
    let out = ctx.out;
    let timeout = ctx.timeout();
    let addr = resolve(spec, 10800)?;
    let mut tracker = within(
        timeout,
        Tracker::login(addr, &Login::default(), ctx.table()?),
    )
    .await?;
    match cmd {
        TrackerCmd::Groups => {
            let groups = within(timeout, tracker.groups()).await?;
            let rows: Vec<Vec<serde_json::Value>> = groups
                .iter()
                .map(|g| {
                    vec![
                        json!(lossy(&g.name)),
                        json!(g.servers),
                        json!(lossy(&g.description)),
                    ]
                })
                .collect();
            out.table("group", &["name", "servers", "description"], &rows);
        }
        TrackerCmd::Servers { group } => {
            let group = match group {
                Some(g) => g.into_bytes(),
                None => within(timeout, tracker.groups())
                    .await?
                    .first()
                    .map(|g| g.name.clone())
                    .unwrap_or_default(),
            };
            let servers = within(timeout, tracker.servers(&group)).await?;
            let rows: Vec<Vec<serde_json::Value>> = servers
                .iter()
                .map(|s| {
                    vec![
                        json!(s.socket_addr().to_string()),
                        json!(lossy(&s.name)),
                        json!(s.users),
                        json!(time_string(s.first_seen)),
                        json!(lossy(&s.description)),
                    ]
                })
                .collect();
            out.table(
                "server",
                &["address", "name", "users", "first_seen", "description"],
                &rows,
            );
        }
    }
    Ok(())
}
