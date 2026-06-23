// src/main.rs
mod ai;
mod app;
mod logger;
mod ui;
use crate::app::{App, Runner};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use std::io::{self};

#[tokio::main]
pub async fn main() -> io::Result<()> {
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let mut runner = Runner::new();
    let app = App::new(runner.should_exit.clone());
    runner.run(app).await?;
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}
