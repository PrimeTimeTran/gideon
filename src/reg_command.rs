use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "gideon",
    version,
    about = "Gideon is a command-line tool for managing AI agents and their contexts.",
    long_about = None
)]
pub struct Cli {
    #[arg(short, long, global = true)]
    pub verbose: bool,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    Start,
    StartServer,
    StartClient,
}

pub fn parse() -> Cli {
    Cli::parse()
}
