use anyhow::Ok;
use cli::Context;
use rmcp::ServiceExt;
use tokio::io::{self, AsyncBufReadExt, stdin, stdout};

mod backend;
mod cli_process;
mod reg_command;
mod router;
mod runtime;
mod service;

use crate::{
    backend::{server, server::MyServer, tool},
    reg_command::{Cli, Command, parse},
    router::execute,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let server = MyServer;
    let msg = server.hello().await;
    dbg!("{}", msg);
    let msg = server.greeting().await;
    dbg!("{:?}", msg);
    let result = server
        .code_review(rmcp::handler::server::wrapper::Parameters(
            server::CodeReviewArgs {
                language: "Rust".into(),
                focus_areas: Some(vec!["performance".into()]),
            },
        ))
        .await;
    dbg!("{:?}", result);

    Ok(())
}
