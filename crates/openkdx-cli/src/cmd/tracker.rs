use openkdx_client::tracker::Tracker;
use serde_json::json;

use super::{Ctx, resolve};
use crate::args::TrackerCmd;
use crate::out::{lossy, time_string};

pub async fn tracker(ctx: &mut Ctx, spec: &str, cmd: TrackerCmd) -> anyhow::Result<()> {
    let out = ctx.out;
    let addr = resolve(spec, 10800)?;
    let tracker = Tracker::connect(addr, &ctx.table()?, ctx.globals.timeout()).await?;
    match cmd {
        TrackerCmd::Groups => {
            let rows: Vec<Vec<serde_json::Value>> = tracker
                .groups()
                .await?
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
                None => tracker
                    .groups()
                    .await?
                    .first()
                    .map(|g| g.name.clone())
                    .unwrap_or_default(),
            };
            let rows: Vec<Vec<serde_json::Value>> = tracker
                .servers(&group)
                .await?
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
    tracker.close().await;
    Ok(())
}
