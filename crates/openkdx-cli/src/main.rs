#![forbid(unsafe_code)]
//! `openkdx`: a thin command-line client over `kdx-client`.

mod args;
mod cmd;
mod out;

use clap::Parser;
use kdx_client::ServerError;
use kdx_proto::handshake::HandshakeError;

use crate::args::Cli;
use crate::cmd::{Ctx, Disconnected, NoTable};

/// Exit codes: 1 usage or local error, 2 login failed, no table or server
/// error, 3 disconnected by the server, 4 timeout.
fn exit_code(err: &anyhow::Error) -> i32 {
    if err.downcast_ref::<HandshakeError>().is_some()
        || err.downcast_ref::<ServerError>().is_some()
        || err.downcast_ref::<NoTable>().is_some()
    {
        2
    } else if err.downcast_ref::<Disconnected>().is_some() {
        3
    } else if err.downcast_ref::<tokio::time::error::Elapsed>().is_some() {
        4
    } else {
        1
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
