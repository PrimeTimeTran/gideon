use cli::CliCommand;
use cli::Context;

use crate::reg_command::{Cli, Command};
use crate::runtime::GideonCLIProcess;

pub async fn execute(cli: Cli, ctx: Context) {
    match cli.command {
        Command::Start => GideonCLIProcess.run(&ctx).await,
        _ => todo!("Todo: implement other commands {:?}", cli.command),
    }
}
