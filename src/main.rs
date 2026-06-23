use anyhow::Ok;
use cli::Context;
use rmcp::ServiceExt;

use tokio::{
    io::{self, AsyncBufReadExt, stdin, stdout},
    sync::mpsc,
};

mod backend;
mod cli_process;
mod context;
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
    let cli = parse();
    let ctx = Context {
        verbose: cli.verbose,
    };

    run_mcp_logic().await?;

    Ok(())
}

async fn run_mcp_logic() -> anyhow::Result<()> {
    let (tx, mut rx) = mpsc::channel::<String>(32);
    tokio::spawn(async move {
        let server = MyServer;
        while let Some(command) = rx.recv().await {
            match command.as_str() {
                "hello" => {
                    let msg = server.hello().await;
                    dbg!("Server says: {:?}", msg);
                }
                "review" => {
                    let result = server
                        .code_review(rmcp::handler::server::wrapper::Parameters(
                            server::CodeReviewArgs {
                                language: "Rust".into(),
                                focus_areas: Some(vec!["performance".into()]),
                            },
                        ))
                        .await;
                    dbg!("Review result: {:?}", result);
                }
                _ => println!("Unknown command"),
            }
        }
    });

    println!("MCP Shell Started. Type 'hello', 'review', or 'exit'.");

    let mut reader = tokio::io::BufReader::new(tokio::io::stdin());
    let mut line = String::new();

    loop {
        line.clear();
        reader.read_line(&mut line).await?;
        let input = line.trim().to_string();

        if input == "exit" {
            break;
        }

        let _ = tx.send(input).await;
    }

    Ok(())
}
