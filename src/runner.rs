use crate::{app::App, ui::render_ui};

use crossterm::event::{self};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::{
    io::{self, Stdout, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub struct Runner {
    pub terminal: Terminal<CrosstermBackend<Stdout>>,
    _guard: TerminalGuard,
    pub should_exit: Arc<AtomicBool>,
}

impl Default for Runner {
    fn default() -> Self {
        Self::new()
    }
}

impl Runner {
    pub fn new() -> Self {
        let should_exit = Arc::new(AtomicBool::new(false));
        let q = should_exit.clone();
        tokio::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;
            q.store(true, Ordering::SeqCst);
        });
        std::panic::set_hook(Box::new(|info| {
            ratatui::restore();
            eprintln!("Panic occurred: {:?}", info);
        }));

        let _guard: TerminalGuard = TerminalGuard;
        let terminal = ratatui::init();
        Self {
            terminal,
            _guard,
            should_exit: Arc::new(AtomicBool::new(false)),
        }
    }

    pub async fn run(&mut self, mut app: App) -> io::Result<()> {
        let _guard = &self._guard;
        loop {
            if self.should_exit.load(Ordering::SeqCst) || app.should_exit.load(Ordering::SeqCst) {
                break;
            }
            app.tick();
            if event::poll(std::time::Duration::from_millis(150))? {
                match app.handle_events() {
                    Ok(false) => {
                        app.handle_exit();
                        break;
                    }
                    Err(_) => {
                        app.handle_exit();
                        break;
                    }
                    Ok(true) => {}
                }
            }
            self.terminal.draw(|f| {
                render_ui(f, &mut app);
            })?;
        }
        Ok(())
    }
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        ratatui::restore();
        let _ = io::stdout().flush();
    }
}
