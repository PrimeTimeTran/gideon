use gideon::{
    Agent, App, Runner,
    agent::{new_agent_system, run_agent_manager},
};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
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
