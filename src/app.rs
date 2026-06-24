use crate::{
    ai::{AgentStatus, prompt_ollama, run_agent_loop},
    logger::Logger,
    ui::{UiState, render_ui},
};
use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseEventKind};

use ratatui::{Terminal, backend::CrosstermBackend, layout::Rect, widgets::ListState};
use std::{
    io::{self, Stdout, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};

pub struct Runner {
    pub terminal: Terminal<CrosstermBackend<Stdout>>,
    _guard: TerminalGuard,
    pub(crate) should_exit: Arc<AtomicBool>,
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
        // app.scroll_to_bottom();
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

#[derive(Clone)]
pub enum AppMode {
    Normal,
    Command,
    Editing,
    LeaderPending,
}
#[derive(PartialEq, Clone)]
pub enum AgentMode {
    Waiting,
    Thinking,
    Error(String),
}

#[derive(Clone)]
pub struct KeyConfig {
    next_tab: (KeyCode, KeyModifiers),
    prev_tab: (KeyCode, KeyModifiers),
}

impl Default for KeyConfig {
    fn default() -> Self {
        Self {
            next_tab: (KeyCode::Char('l'), KeyModifiers::CONTROL),
            prev_tab: (KeyCode::Char('h'), KeyModifiers::CONTROL),
        }
    }
}

#[derive(PartialEq, Clone)]
pub struct Message {
    role: Role,
    content: String,
}
#[derive(PartialEq, Clone)]
enum Role {
    User,
    AI,
}

pub struct App {
    pub is_initialized: bool,
    pub offset: i32,
    pub ai_list_state: ListState,
    pub user_list_state: ListState,
    pub ai_area: Rect,
    pub user_area: Rect,

    pub should_exit: Arc<AtomicBool>,
    pub cursor_visible: bool,
    pub last_cursor_toggle: std::time::Instant,

    pub tabs: Vec<String>,
    pub active_tab: usize,
    pub key_config: KeyConfig,
    pub input_buffer: String,

    pub history: Vec<String>,
    pub user_history: Vec<String>,
    pub ai_history: Vec<String>,
    pub logger: Logger,
    pub logs: Vec<String>,
    pub messages: Vec<Message>,

    pub mode: AppMode,
    pub agent_mode: AgentMode,
    pub tx: tokio::sync::mpsc::UnboundedSender<AgentStatus>,
    pub rx: tokio::sync::mpsc::UnboundedReceiver<AgentStatus>,
    pub spinner_index: usize,
}

impl App {
    pub fn new(should_exit: Arc<AtomicBool>) -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<AgentStatus>();
        let session_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let logger = Logger::new(session_id);
        let (history_u, history_a) = logger.load_history();
        let mut combined_history = history_u.clone();
        combined_history.extend(history_a.clone());
        Self {
            tx,
            rx,
            is_initialized: false,
            user_list_state: ListState::default(),
            ai_list_state: ListState::default(),
            spinner_index: 0,
            tabs: vec!["Chat".into(), "Config".into(), "Logs".into()],
            active_tab: 0,
            should_exit,
            key_config: KeyConfig::default(),
            input_buffer: String::new(),
            user_history: history_u,
            ai_history: history_a,
            offset: 0,
            ai_area: Rect::default(),
            user_area: Rect::default(),
            history: combined_history,
            logger,
            messages: Vec::new(),
            logs: Vec::new(),
            mode: AppMode::Normal,
            agent_mode: AgentMode::Waiting,
            cursor_visible: true,
            last_cursor_toggle: std::time::Instant::now(),
        }
    }

    fn next_tab(&mut self) {
        self.active_tab = (self.active_tab + 1) % self.tabs.len();
    }
    fn prev_tab(&mut self) {
        self.active_tab = (self.active_tab + self.tabs.len() - 1) % self.tabs.len();
    }
    fn perform_special_quit(&mut self) {
        self.active_tab = (self.active_tab + self.tabs.len() - 1) % self.tabs.len();
    }

    fn history_mode(&mut self) {
        self.active_tab = (self.active_tab + self.tabs.len() - 1) % self.tabs.len();
    }

    fn handle_exit(&self) {
        println!("Exiting");
        ratatui::restore();
        print!("\x1b[?25h\x1b[0m");
        let _ = std::io::stdout().flush();
    }
    fn handle_command(&mut self, input: &str) {
        match input {
            ":q" => self.should_exit = Arc::new(AtomicBool::new(true)),
            ":history" => self.history_mode(),
            _ => self.logs.push("Unknown command".to_string()),
        }
    }
    fn handle_submit(&mut self) {
        if self.input_buffer.is_empty() {
            return;
        }

        let input = self.input_buffer.clone();
        self.history.push(input.clone());
        self.user_history.push(input.clone());
        self.input_buffer.clear();
        let _ = self.logger.log_to_file("user", &input);

        if input.starts_with(':') {
            self.handle_command(&input);
        } else {
            self.agent_mode = AgentMode::Thinking;
            let tx = self.tx.clone();

            let input_text = input.clone();

            tokio::spawn(async move {
                match run_agent_loop(input_text, tx.clone()).await {
                    Ok(_) => {
                        // Signal completion to the UI
                        // let _ = tx.send(AgentStatus::Finished(
                        //     "Action executed successfully.".to_string(),
                        // ));
                    }
                    Err(e) => {
                        // Signal error to the UI
                        let _ = tx.send(AgentStatus::Error(e.to_string()));
                    }
                }
            });
        }
        self.mode = AppMode::Normal;
    }
    fn update_cursor_blink(&mut self) {
        if self.last_cursor_toggle.elapsed().as_millis() > 500 {
            self.cursor_visible = !self.cursor_visible;
            self.last_cursor_toggle = std::time::Instant::now();
        }
    }

    pub fn scroll_all_to_bottom(&mut self, width: u16) {
        if self.is_initialized {
            return;
        }

        use textwrap::{Options, wrap};
        let wrap_opts = Options::new(width.saturating_sub(4) as usize);

        let ai_lines: usize = self
            .ai_history
            .iter()
            .map(|content| wrap(content, &wrap_opts).len() + 1)
            .sum();

        let user_lines: usize = self
            .user_history
            .iter()
            .map(|content| wrap(content, &wrap_opts).len() + 1)
            .sum();
        *self.ai_list_state.offset_mut() = ai_lines.saturating_sub(20);
        *self.user_list_state.offset_mut() = user_lines.saturating_sub(20);

        self.is_initialized = true;
    }

    fn apply_scroll(state: &mut ListState, delta: i32) {
        let offset = state.offset() as i32;
        let new_offset = (offset + delta).max(0) as usize;
        *state.offset_mut() = new_offset;
    }

    fn scroll_list(&mut self, mouse_pos: (u16, u16), delta: i32) {
        if self.ai_area.contains(mouse_pos.into()) {
            Self::apply_scroll(&mut self.ai_list_state, delta);
        } else if self.user_area.contains(mouse_pos.into()) {
            Self::apply_scroll(&mut self.user_list_state, delta);
        }
    }
    fn handle_events(&mut self) -> anyhow::Result<bool> {
        let event = event::read()?;
        if let Event::Mouse(mouse) = event {
            let pos = (mouse.column, mouse.row);
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    if self.ai_area.contains(pos.into()) || self.user_area.contains(pos.into()) {
                        self.scroll_list(pos, -1);
                    }
                }
                MouseEventKind::ScrollDown
                    if (self.ai_area.contains(pos.into())
                        || self.user_area.contains(pos.into())) =>
                {
                    self.scroll_list(pos, 1);
                }
                _ => {}
            }
            return Ok(true);
        } else if let Event::Key(key) = event {
            match self.mode {
                AppMode::Normal => match key.code {
                    KeyCode::Enter => self.handle_submit(),
                    KeyCode::Char(':') if self.input_buffer.is_empty() => {
                        self.input_buffer.push(':');
                        self.mode = AppMode::Command;
                    }
                    KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        return Ok(false);
                    }
                    KeyCode::Backspace => {
                        self.input_buffer.pop();
                    }

                    _ if key.code == self.key_config.next_tab.0
                        && key.modifiers.contains(self.key_config.next_tab.1) =>
                    {
                        self.next_tab();
                    }
                    _ if key.code == self.key_config.prev_tab.0
                        && key.modifiers.contains(self.key_config.prev_tab.1) =>
                    {
                        self.prev_tab();
                    }

                    KeyCode::Char(c) => {
                        self.input_buffer.push(c);
                    }

                    _ => {}
                },
                AppMode::Command => match key.code {
                    KeyCode::Enter => self.handle_submit(),
                    KeyCode::Backspace => {
                        self.input_buffer.pop();
                        if self.input_buffer.is_empty() {
                            self.mode = AppMode::Normal;
                        }
                    }
                    KeyCode::Esc => {
                        self.input_buffer.clear();
                        self.mode = AppMode::Normal;
                    }
                    KeyCode::Char(c) => {
                        self.input_buffer.push(c);
                    }
                    _ => {}
                },
                AppMode::LeaderPending => match key.code {
                    KeyCode::Char('h') => self.history_mode(),
                    _ => self.mode = AppMode::Normal,
                },
                _ => {}
            }
        }
        Ok(true)
    }
    pub fn tick(&mut self) {
        self.update_cursor_blink();

        if self.agent_mode == AgentMode::Thinking {
            self.spinner_index += 1;
        }

        while let Ok(status) = self.rx.try_recv() {
            match status {
                AgentStatus::Thinking => {
                    self.agent_mode = AgentMode::Thinking;
                }
                AgentStatus::Working(msg) => {
                    self.logs.push(msg);
                }
                AgentStatus::Finished(answer) => {
                    let _ = self.logger.log_to_file("ai", &answer);
                    self.agent_mode = AgentMode::Waiting;
                    self.ai_history.push(answer);
                }
                AgentStatus::Error(e) => {
                    self.agent_mode = AgentMode::Error(e);
                }
            }
        }
    }
    pub fn get_ui_data(&self) -> UiState<'_> {
        UiState {
            user_history: &self.user_history,
            ai_history: &self.ai_history,
            mode: &self.mode,
            history: &self.history,
            messages: &self.messages,
            logs: &self.logs,
            tabs: &self.tabs,
            active_tab: self.active_tab,
            input_buffer: &self.input_buffer,
            agent_mode: &self.agent_mode,
            spinner_index: self.spinner_index,
            cursor_visible: self.cursor_visible,
        }
    }
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        dbg!("Quitting");
        ratatui::restore();
        let _ = io::stdout().flush();
    }
}
