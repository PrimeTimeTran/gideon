use async_trait::async_trait;
use cli::{CliCommand, Context};

use rmcp::{ServiceExt, transport::TokioChildProcess};
use tokio::io::{stdin, stdout};
use tokio::process::Command;

use crate::context::{
    bootstrap_runtime, load_crate_ai_context, load_root_client_context, resolve_active_crates,
};
use crate::{cli_process::run_my_ui, server::MyServer};

pub struct GideonCLIProcess;

#[async_trait]
impl CliCommand for GideonCLIProcess {
    async fn run(&self, ctx: &Context) {
        self.check4(ctx).await;
    }
}

impl GideonCLIProcess {
    /// Checkpoint #1:
    /// Perform analysis of workspace collecting context works.
    async fn check1(&self, ctx: &Context) {
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

    /// Checkpoint #2:
    /// GideonCLIProcess run creates client MCP service and calls list_all_tools method.
    /// - The run method should execute the command "pnpm dlx @modelcontextprotocol/server-everything" using TokioChildProcess.
    /// - After starting the process, it should create a client and call the list_all_tools method.
    /// - Handle any errors that occur during the process execution and print
    async fn check2(&self, _ctx: &Context) {
        let result: Result<(), Box<dyn std::error::Error>> = async {
            let mut cmd = Command::new("pnpm");
            cmd.args(["dlx", "@modelcontextprotocol/server-everything"]);
            let transport = TokioChildProcess::new(cmd)?;
            let client = ().serve(transport).await?;
            use rmcp::model::CallToolRequestParams;
            let tools = client.list_all_tools().await?;
            dbg!("Available tools: {:?}", tools);
            let result = client.call_tool(CallToolRequestParams::new("add")).await?;
            tokio::signal::ctrl_c().await?;
            Ok(())
        }
        .await;
        if let Err(e) = result {
            eprintln!("Error running GideonCLIProcess: {:?}", e);
        }
    }

    /// Checkpoint #3:
    /// Tokio/mcp processes conflict.
    async fn check3(&self, _ctx: &Context) {
        let result: Result<(), Box<dyn std::error::Error>> = async {
            let server = MyServer;

            let transport = (stdin(), stdout());
            if let Err(e) = server.serve(transport).await {
                eprintln!("MCP Server error: {:?}", e);
            }
            dbg!("Gideon started");
            Ok(())
        }
        .await;
        dbg!("Gideon started");

        if let Err(e) = result {
            eprintln!("Error running GideonCLIProcess: {:?}", e);
        }
    }
}

impl GideonCLIProcess {
    async fn check4(&self, _ctx: &Context) {
        run_my_ui().await;
    }
}

#[cfg(test)]
mod tests {
    use rmcp::handler::server::wrapper::Parameters;

    use crate::tool::AddVars;

    use super::*;

    #[test]
    fn test_wiring() {
        let server = MyServer;
        let vars = AddVars { a: 10, b: 20 };
        let result = server.add(Parameters(vars));
        assert_eq!(result, "30");
        println!("✅ Wiring confirmed: 10 + 20 = {}", result);
    }
}
