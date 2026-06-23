use cli::CliCommand;
use cli::Context;

use crate::reg_command::{Cli, Command};
use crate::runtime::AgentContext;
use crate::runtime::{Client, GideonCLIProcess};

pub async fn execute(cli: Cli, ctx: Context) {
    match cli.command {
        // Command::Start => Client.run(&ctx).await,
        Command::Start => GideonCLIProcess.run(&ctx).await,
        // Command::StartServer => Server.run(&ctx).await,
        // Command::StartClient => Client.run(&ctx).await,
        _ => todo!("Todo: implement other commands {:?}", cli.command),
    }
}
