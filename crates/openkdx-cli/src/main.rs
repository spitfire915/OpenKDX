#![forbid(unsafe_code)]
//! `openkdx`: a thin command-line client over `openkdx-client`.

mod args;
mod cmd;
mod out;

use clap::Parser;
use openkdx_client::ClientError;

use crate::args::Cli;
use crate::cmd::Ctx;

/// Exit codes: 1 usage or local error, 2 login failed or server error,
/// 3 disconnected by the server, 4 timeout.
fn exit_code(err: &anyhow::Error) -> i32 {
    match err.downcast_ref::<ClientError>() {
        Some(ClientError::LoginFailed(_))
        | Some(ClientError::ProofMismatch)
        | Some(ClientError::Server { .. }) => 2,
        Some(ClientError::Disconnected) | Some(ClientError::DisconnectedBy(_)) => 3,
        Some(ClientError::Timeout) => 4,
        _ => 1,
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let mut ctx = Ctx::new(cli.globals);
    let result = cmd::dispatch(&mut ctx, cli.command).await;
    ctx.close().await;
    if let Err(e) = result {
        eprintln!("openkdx: {e:#}");
        std::process::exit(exit_code(&e));
    }
}
