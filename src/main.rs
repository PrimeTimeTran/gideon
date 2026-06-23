use anyhow::Error;
use colored::*;
use crossterm::{
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyModifiers, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    DefaultTerminal, Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs},
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    fs::{File, OpenOptions},
    io::{self, BufRead, BufReader, Stdout, Write},
    path::Path,
    rc::Rc,
    sync::Mutex,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct UiState<'a> {
    pub messages: &'a [Message],
    pub mode: &'a AppMode,
    pub history: &'a [String],
    pub logs: &'a [String],
    pub tabs: &'a [String],
    pub active_tab: usize,
    pub input_buffer: &'a str,
    pub agent_mode: &'a AgentMode,
    pub spinner_index: usize,
    pub cursor_visible: bool,
    pub user_history: &'a [String],
    pub ai_history: &'a [String],
}

struct Runner {
    pub terminal: Terminal<CrosstermBackend<Stdout>>,
    _guard: TerminalGuard,
    should_exit: Arc<AtomicBool>,
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
struct KeyConfig {
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
    offset: i32,
    ai_list_state: ListState,
    user_list_state: ListState,
    pub ai_area: Rect,
    pub user_area: Rect,

    should_exit: Arc<AtomicBool>,
    cursor_visible: bool,
    last_cursor_toggle: std::time::Instant,

    tabs: Vec<String>,
    active_tab: usize,
    key_config: KeyConfig,
    input_buffer: String,

    history: Vec<String>,
    user_history: Vec<String>,
    ai_history: Vec<String>,
    logger: Logger,
    logs: Vec<String>,
    messages: Vec<Message>,

    mode: AppMode,
    agent_mode: AgentMode,
    pub tx: broadcast::Sender<Result<String, String>>,
    pub rx: broadcast::Receiver<Result<String, String>>,
    spinner_index: usize,
}

impl App {
    fn new(should_exit: Arc<AtomicBool>) -> Self {
        let (tx, _) = broadcast::channel::<Result<String, String>>(16);
        let rx = tx.subscribe();
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
        self.input_buffer.clear();
        let _ = self.logger.log_to_file("user", &input);

        if input.starts_with(':') {
            self.handle_command(&input);
        } else {
            self.agent_mode = AgentMode::Thinking;

            let tx = self.tx.clone();
            tokio::spawn(async move {
                let result = prompt_ollama(input).await.map_err(|e| e.to_string());
                let _ = tx.send(result);
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
    // fn handle_events(&mut self) -> anyhow::Result<bool> {
    //     if let Event::Key(key) = event::read()? {
    //         match self.mode {
    //             AppMode::Normal => match key.code {
    //                 KeyCode::Enter => self.handle_submit(),
    //                 KeyCode::Char(':') if self.input_buffer.is_empty() => {
    //                     self.input_buffer.push(':');
    //                     self.mode = AppMode::Command;
    //                 }
    //                 KeyCode::Char('q') if key.modifiers.contains(KeyModifiers::CONTROL) => {
    //                     return Ok(false);
    //                 }
    //                 KeyCode::Backspace => {
    //                     self.input_buffer.pop();
    //                 }

    //                 _ if key.code == self.key_config.next_tab.0
    //                     && key.modifiers.contains(self.key_config.next_tab.1) =>
    //                 {
    //                     self.next_tab();
    //                 }
    //                 _ if key.code == self.key_config.prev_tab.0
    //                     && key.modifiers.contains(self.key_config.prev_tab.1) =>
    //                 {
    //                     self.prev_tab();
    //                 }

    //                 KeyCode::Char(c) => {
    //                     self.input_buffer.push(c);
    //                 }

    //                 _ => {}
    //             },
    //             AppMode::Command => match key.code {
    //                 KeyCode::Enter => self.handle_submit(),
    //                 KeyCode::Backspace => {
    //                     self.input_buffer.pop();
    //                     if self.input_buffer.is_empty() {
    //                         self.mode = AppMode::Normal;
    //                     }
    //                 }
    //                 KeyCode::Esc => {
    //                     self.input_buffer.clear();
    //                     self.mode = AppMode::Normal;
    //                 }
    //                 KeyCode::Char(c) => {
    //                     self.input_buffer.push(c);
    //                 }
    //                 _ => {}
    //             },
    //             AppMode::LeaderPending => match key.code {
    //                 KeyCode::Char('h') => self.history_mode(),
    //                 _ => self.mode = AppMode::Normal,
    //             },
    //             _ => {}
    //         }
    //     }
    //     Ok(true)
    // }
    pub fn tick(&mut self) {
        self.update_cursor_blink();

        if self.agent_mode == AgentMode::Thinking {
            self.spinner_index += 1;
        }

        match self.rx.try_recv() {
            Ok(result) => {
                self.agent_mode = AgentMode::Waiting;

                match result {
                    Ok(answer) => {
                        let _ = self.logger.log_to_file("ai", &answer);

                        self.messages.push(Message {
                            role: Role::AI,
                            content: answer,
                        });
                    }
                    Err(e) => {
                        self.agent_mode = AgentMode::Error(e.to_string());
                        self.logs.push(format!("AI Error: {}", e));
                    }
                }
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                // No message, nothing to do
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => {
                // Optional: Handle if we missed messages
                self.logs.push(format!("Warning: Missed {} messages", n));
            }
            Err(tokio::sync::broadcast::error::TryRecvError::Closed) => {
                // Channel closed, might want to handle this
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

#[tokio::main]
async fn main() -> io::Result<()> {
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let mut runner = Runner::new();
    let app = App::new(runner.should_exit.clone());
    runner.run(app).await?;
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

pub fn render_ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(f.area());

    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    app.ai_area = content_chunks[0];
    app.user_area = content_chunks[1];
    let (ai_area, user_area) = (app.ai_area, app.user_area);

    // 1. Render all widgets that require mutable app access FIRST
    render_conversation(f, app, ai_area);
    render_history(f, app, user_area);

    // 2. Scope the snapshot so it is dropped after these widgets are drawn
    {
        let state = app.get_ui_data();
        render_tabs(f, &state, chunks[0]);
        f.render_widget(create_hint_widget(state.clone()), chunks[2]);
        f.render_widget(create_input_widget(state), chunks[3]);
    }
}

fn render_tabs(f: &mut Frame, state: &UiState, area: Rect) {
    f.render_widget(
        Tabs::new(state.tabs.iter().map(|s| s.as_str()))
            .select(state.active_tab)
            .block(Block::default().borders(Borders::ALL).title(" Gideon ")),
        area,
    );
}
// Note: Changed from UiState to &mut App to access the stateful list states
fn render_conversation(f: &mut Frame, app: &mut App, area: Rect) {
    use textwrap::{Options, wrap};
    let width = area.width.saturating_sub(4) as usize;

    let msg_items: Vec<ListItem> = app
        .ai_history
        .iter()
        .flat_map(|content| {
            let wrapped_lines = wrap(content, Options::new(width));
            wrapped_lines
                .into_iter()
                .map(|line| ListItem::new(line.into_owned()))
                .chain(std::iter::once(ListItem::new("")))
        })
        .collect();

    // let list = List::new(msg_items).block(
    //     Block::default()
    //         .borders(Borders::ALL)
    //         .title(" AI Responses "),
    // );

    let list = List::new(msg_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" AI Responses "),
    );

    // CRITICAL: Use render_stateful_widget instead of render_widget
    f.render_stateful_widget(list, area, &mut app.ai_list_state);
}

fn render_history(f: &mut Frame, app: &mut App, area: Rect) {
    use textwrap::{Options, wrap};
    let width = area.width.saturating_sub(4) as usize;

    let hist_items: Vec<ListItem> = app
        .user_history
        .iter()
        .flat_map(|content| {
            let wrapped_lines = wrap(content, Options::new(width));
            wrapped_lines
                .into_iter()
                .map(|line| ListItem::new(line.into_owned()))
                .chain(std::iter::once(ListItem::new("")))
        })
        .collect();

    let list = List::new(hist_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Your Prompts "),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut app.user_list_state);
}
// fn render_conversation(f: &mut Frame, state: &UiState, area: Rect) {
//     use textwrap::{Options, wrap};

//     let width = area.width.saturating_sub(4) as usize;

//     // Now directly using state.ai_history
//     let msg_items: Vec<ListItem> = state
//         .ai_history
//         .iter()
//         .flat_map(|content| {
//             let wrapped_lines = wrap(content, Options::new(width));
//             wrapped_lines
//                 .into_iter()
//                 .map(|line| ListItem::new(line.into_owned()))
//                 .chain(std::iter::once(ListItem::new("")))
//         })
//         .collect();

//     f.render_widget(
//         List::new(msg_items).block(
//             Block::default()
//                 .borders(Borders::ALL)
//                 .title(" AI Responses "),
//         ),
//         area,
//     );
// }

// fn render_history(f: &mut Frame, state: &UiState, area: Rect) {
//     use textwrap::{Options, wrap};
//     let width = area.width.saturating_sub(4) as usize;

//     // Now directly using state.user_history
//     let hist_items: Vec<ListItem> = state
//         .user_history
//         .iter()
//         .flat_map(|content| {
//             let wrapped_lines = wrap(content, Options::new(width));
//             wrapped_lines
//                 .into_iter()
//                 .map(|line| ListItem::new(line.into_owned()))
//                 .chain(std::iter::once(ListItem::new("")))
//         })
//         .collect();

//     f.render_widget(
//         List::new(hist_items)
//             .block(
//                 Block::default()
//                     .borders(Borders::ALL)
//                     .title(" Your Prompts "),
//             )
//             .highlight_symbol(">> "),
//         area,
//     );
// }
fn create_hint_widget(state: UiState<'_>) -> Paragraph<'_> {
    let hint_text = match state.mode {
        AppMode::Normal => "Esc to Quit | I to Edit | Enter to Send",
        AppMode::Editing => "Esc to Normal | Ctrl+S to Save",
        _ => "hi",
    };

    Paragraph::new(hint_text)
        .block(Block::default().borders(Borders::ALL).title(" Hints "))
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::DIM),
        )
        .alignment(Alignment::Center)
}
fn create_input_widget<'a>(state: UiState) -> Paragraph<'a> {
    let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let prefix = if state.agent_mode == &AgentMode::Thinking {
        spinner_chars[state.spinner_index % spinner_chars.len()].to_string()
    } else if !state.input_buffer.is_empty()
        && matches!(
            state.input_buffer.chars().next().unwrap(),
            ':' | '!' | '/' | '.' | '#' | '@'
        )
    {
        state.input_buffer.chars().next().unwrap().to_string()
    } else {
        ">".to_string()
    };
    let mut spans = vec![Span::styled(
        format!("{} ", prefix),
        Style::default().fg(Color::Yellow).bold(),
    )];

    let content = if state.agent_mode == &AgentMode::Thinking {
        "Thinking...".to_string()
    } else {
        state.input_buffer.to_string()
    };

    spans.push(Span::styled(content, Style::default().fg(Color::White)));
    if state.agent_mode != &AgentMode::Thinking && state.cursor_visible {
        spans.push(Span::styled("_", Style::default().fg(Color::Yellow).bold()));
    }
    let line = Line::from(spans);
    Paragraph::new(line).block(Block::default().borders(Borders::ALL).title(" Input "))
}

#[derive(Deserialize, Debug)]
struct OllamaResponse {
    done: bool,
    model: String,
    response: String,
    created_at: String,
}

async fn run_agent_loop(user_input: String) -> anyhow::Result<()> {
    let ollama_res = prompt_ollama_for_json(user_input).await?;
    let json_content = ollama_res.response.trim();
    match serde_json::from_str::<AgentCommand>(json_content) {
        Ok(cmd) => match cmd {
            AgentCommand::WriteFile { path, content } => {
                if path.starts_with("./allowed_dir/") {
                    std::fs::write(path, content)?;
                    println!("Action executed successfully.");
                } else {
                    println!("Security error: Access denied to path.");
                }
            }
            AgentCommand::ReadFile { path } => {
                println!("AI: {}", path)
            }
            AgentCommand::Chat { message } => println!("AI: {}", message),
            // _ => {
            //     todo!("unhandled")
            // }
        },
        Err(e) => {
            // TODO: Handle cases where the LLM talks instead of returning JSON
            eprintln!(
                "Failed to parse command from AI: {}. Raw text: {}",
                e, json_content
            );
        }
    }
    Ok(())
}
async fn prompt_ollama_for_json(user_input: String) -> anyhow::Result<OllamaResponse> {
    use reqwest::Client;
    let client = Client::new();
    let url = "http://localhost:11434/api/generate";

    let payload = json!({
        "model": "qwen3:8b",
        "prompt": format!("{} Respond with only a valid JSON object.", user_input),
        "stream": false,
        "format": "json"
    });
    let value = client
        .post(url)
        .json(&payload)
        .send()
        .await?
        .json::<OllamaResponse>()
        .await?;
    anyhow::Ok(value)
}
async fn prompt_ollama(user_input: String) -> anyhow::Result<String> {
    use reqwest::Client;
    let client = Client::new();
    let url = "http://localhost:11434/api/generate";

    let payload = json!({
        "model": "qwen3:8b",
        "prompt": format!("{} Respond with only the direct answer.", user_input),
        "stream": false
    });

    let res = client
        .post(url)
        .json(&payload)
        .send()
        .await?
        .json::<OllamaResponse>()
        .await?;

    anyhow::Ok(res.response.trim().to_string())
}

pub fn print_user(input: &str) {
    println!("{}: {}", "You".blue().bold(), input);
}

pub fn print_ai(answer: &str) {
    println!("{}: {}", "AI".green().bold(), answer);
}

pub fn print_system(msg: &str) {
    println!("{} {}", "::".yellow(), msg.italic());
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        dbg!("Quitting");
        ratatui::restore();
        let _ = io::stdout().flush();
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct HistoryEntry {
    session: u64,
    role: String,
    content: String,
    timestamp: u64,
}

#[derive(Serialize, Deserialize, Clone)]
struct LogEntry {
    session: u64,
    role: String,
    content: String,
    timestamp: u64,
}

#[derive(Clone)]
struct Logger {
    session_id: u64,
}

impl Logger {
    fn new(session_id: u64) -> Self {
        Self { session_id }
    }
    pub fn log_to_file(&self, role: &str, content: &str) -> io::Result<()> {
        let entry = LogEntry {
            session: self.session_id,
            role: role.to_string(),
            content: content.to_string(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        };

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open("chat_log.jsonl")?;

        // Serialize the unified struct
        let json = serde_json::to_string(&entry).map_err(io::Error::other)?;
        writeln!(file, "{}", json)?;
        Ok(())
    }

    pub fn load_history(&self) -> (Vec<String>, Vec<String>) {
        let path = "chat_log.jsonl";
        if !Path::new(path).exists() {
            return (Vec::new(), Vec::new());
        }

        let mut user_h = Vec::new();
        let mut ai_h = Vec::new();

        let file = match File::open(path) {
            Ok(f) => f,
            Err(_) => return (Vec::new(), Vec::new()),
        };

        let reader = BufReader::new(file);
        for line in reader.lines() {
            if let Ok(l) = line {
                if let Ok(entry) = serde_json::from_str::<LogEntry>(&l) {
                    match entry.role.as_str() {
                        "user" => user_h.push(entry.content),
                        "ai" => ai_h.push(entry.content),
                        _ => {}
                    }
                }
            }
        }

        (user_h, ai_h)
    }
}

#[derive(serde::Deserialize)]
#[serde(tag = "action", content = "params")]
enum AgentCommand {
    WriteFile { path: String, content: String },
    ReadFile { path: String },
    Chat { message: String },
}

async fn get_command_with_retry(user_input: String) -> anyhow::Result<AgentCommand> {
    for _ in 0..3 {
        let res = prompt_ollama_for_json(user_input.clone()).await?;
        if let Ok(cmd) = serde_json::from_str::<AgentCommand>(&res.response) {
            return Ok(cmd);
        }
        // Optionally: append "Previous response was not valid JSON. Please try again."
    }
    anyhow::bail!("AI failed to provide valid JSON after 3 attempts.")
}
