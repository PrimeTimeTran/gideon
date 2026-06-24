mod agent;
mod app;
mod logger;
mod ui;
use gideon::agent::Agent;
use gideon::app::{App, Runner};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use gideon::agent::{new_agent_system, run_agent_manager};
use std::io::{self};

#[tokio::main]
pub async fn main() -> io::Result<()> {
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let mut runner = Runner::new();
    let (bus, runtime, rx) = new_agent_system();
    tokio::spawn(run_agent_manager(runtime));
    let app = App::new(runner.should_exit.clone(), bus, rx);
    runner.run(app).await?;
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}
