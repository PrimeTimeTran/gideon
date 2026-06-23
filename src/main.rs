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
    server
        .code_review(rmcp::handler::server::wrapper::Parameters(
            server::CodeReviewArgs {
                language: "Rust".into(),
                focus_areas: Some(vec!["performance".into()]),
            },
        ))
        .await;
    println!("{}", msg);

    Ok(())
}
// server
//     .serve((tokio::io::stdin(), tokio::io::stdout()))
//     .await?;
// Ok(())
// // let cli = Cli::parse();

// match cli.command {
//     Commands::Start => {
//         // This only runs when the subprocess spawns itself
//         let server = MyServer::new();
//         server
//             .serve((tokio::io::stdin(), tokio::io::stdout()))
//             .await?;
//     }
//     Commands::RunUi => {
//         // Your custom UI logic
//         run_my_ui().await?;
//     }
// }
// Ok(())
