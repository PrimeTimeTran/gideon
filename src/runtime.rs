use async_trait::async_trait;
use cli::{CliCommand, Context};

use rmcp::{
    Service, ServiceExt,
    handler::{client::ClientHandler, server::wrapper::Parameters},
    model::{CallToolRequest, CallToolRequestParams},
    serde_json,
    service::{RoleClient, RunningService},
    transport::{self, ConfigureCommandExt, TokioChildProcess},
};
use std::borrow::Cow;
use std::path::Path;
use tokio::io::{self, AsyncBufReadExt, stdin, stdout};
use tokio::{
    net::{UnixListener, UnixStream},
    {process::Command, sync::oneshot},
};

pub struct GideonCLIProcess;
pub struct GideonCLIProcess2;

use crate::{cli_process::run_my_ui, server::MyServer};

#[async_trait]
impl CliCommand for GideonCLIProcess {
    async fn run(&self, _ctx: &Context) {
        run_my_ui().await.expect("Failed to run Gideon CLI process");
    }
}

#[async_trait]
impl CliCommand for GideonCLIProcess2 {
    async fn run(&self, _ctx: &Context) {
        // loop {
        //     // e.g., perform background maintenance, watch logs, etc.
        //     tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        //     println!("Gideon Server is still active in the background...");
        // }
        // This works, but now we need to send in commands. how?
        // let result: Result<(), Box<dyn std::error::Error>> = async {
        //     use rmcp::transport::stdio;
        //     let server = MyServer::new();
        //     let transport = (stdin(), stdout());
        //     if let Err(e) = server.serve(transport).await {
        //         eprintln!("MCP Server error: {:?}", e);
        //     }
        //     Ok(())
        // }
        // .await;

        // if let Err(e) = result {
        //     eprintln!("Error running GideonCLIProcess: {:?}", e);
        // }
    }
    // Checkpoint #1:
    // GideonCLIProcess run creates client MCP service and calls list_all_tools method.
    // - The run method should execute the command "pnpm dlx @modelcontextprotocol/server-everything" using TokioChildProcess.
    // - After starting the process, it should create a client and call the list_all_tools method.
    // - Handle any errors that occur during the process execution and print
    // async fn _run(&self, _ctx: &Context) {
    //     let result: Result<(), Box<dyn std::error::Error>> = async {
    //         let mut cmd = Command::new("pnpm");
    //         cmd.args(["dlx", "@modelcontextprotocol/server-everything"]);
    //         let transport = TokioChildProcess::new(cmd)?;
    //         let client = ().serve(transport).await?;
    //         use rmcp::model::CallToolRequestParams;
    //         let tools = client.list_all_tools().await?;
    //         dbg!("Available tools: {:?}", tools);
    //         let result = client.call_tool(CallToolRequestParams::new("add")).await?;
    //         tokio::signal::ctrl_c().await?;
    //         Ok(())
    //     }
    //     .await;
    //     if let Err(e) = result {
    //         eprintln!("Error running GideonCLIProcess: {:?}", e);
    //     }
    // }
}

pub struct GideonHandler;
impl ClientHandler for GideonHandler {}

// impl ClientHandler for GideonHandler {
//     async fn run(socket_path: &str, ctx: &Context) -> anyhow::Result<Self> {
//         let client_root = std::env::current_dir().unwrap();
//         println!("✅ [AGENT RUNTIME CONTEXT READY]");
//         bootstrap_runtime(&client_root);
//         Ok(Self)
//     }
// }

pub struct AgentContext;

impl AgentContext {
    pub async fn new(path: &str, ctx: &Context) -> anyhow::Result<Self> {
        // Connect to the server
        let stream = UnixStream::connect(path).await?;
        // let transport = rmcp::transport::json_rpc(stream);
        // let mcp_client = rmcp::service::Service::new(transport);

        // Run your existing local logic
        println!("🏁[AGENT RUNTIME CONTEXT]");
        let client_root = std::env::current_dir().unwrap();

        // Assuming these functions are available in your crate
        // load_root_client_context(&client_root);
        // ... (your existing loop logic)

        println!("✅ [AGENT RUNTIME CONTEXT READY]");
        // bootstrap_runtime(&client_root);

        Ok(Self {})
    }
}

pub struct Client;
impl Client {
    pub async fn run(&self, ctx: &Context) {
        println!("🏁[AGENT RUNTIME CONTEXT]");
        let client_root = std::env::current_dir().unwrap();
        load_root_client_context(&client_root);

        let active_crates = resolve_active_crates(&client_root, ctx);

        for crate_name in active_crates {
            let crate_path = client_root.join("crates").join(&crate_name);
            load_crate_ai_context(&client_root, &crate_path);
        }
        println!("✅ [AGENT RUNTIME CONTEXT READY]");

        bootstrap_runtime(&client_root);
    }
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn load_root_client_context(root: &Path) {
    let path = root.join(".ai").join("agents.md");
    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let title = extract_ai_title(&content);
            println!("  🗂️ [CONTEXT WORKSPACE TITLE] {}", title);
            println!("  🗂️ [CONTEXT WORKSPACE PATH] {}", rel(root, &path));
        }
        Err(_) => {
            println!("⚠️ no workspace .ai/agents.md found");
        }
    }
}

fn load_crate_ai_context(root: &Path, crate_path: &Path) {
    let path = crate_path.join(".ai").join("agents.md");

    match std::fs::read_to_string(&path) {
        Ok(content) => {
            let title = extract_ai_title(&content);
            println!("    📄 [CRATE ENTRY TITLE] {}", title);
            println!("    📄 [CRATE ENTRY PATH] {}", rel(root, &path));
        }
        Err(_) => {
            println!("    ⚠️ crate AI not found");
        }
    }
}

fn resolve_active_crates(_root: &Path, _ctx: &Context) -> Vec<String> {
    vec!["gideon".to_string()]
}

fn extract_ai_title(content: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("# ") {
            return trimmed.trim_start_matches("# ").to_string();
        }
    }

    "Untitled AI Context".to_string()
}

fn bootstrap_runtime(root: &Path) {
    println!("⚙️ bootstrapping runtime from {}", root.display());
}

#[cfg(test)]
mod tests {
    use crate::tool::AddVars;

    use super::*;

    #[test]
    fn test_wiring() {
        let server = MyServer::new();
        let vars = AddVars { a: 10, b: 20 };
        let result = server.add(Parameters(vars));
        assert_eq!(result, "30");
        println!("✅ Wiring confirmed: 10 + 20 = {}", result);
    }
}
