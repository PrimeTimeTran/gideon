use crate::{
    agent::{
        AgentBus, AgentEvent, AgentStatus, RuntimeEvent, SystemEvent, Task, TaskEvent, TaskResult,
    },
    logger::Logger,
    ui::UiState,
};

use crossterm::event::{self, Event, KeyCode, KeyModifiers, MouseEventKind};
use ratatui::{layout::Rect, widgets::ListState};
use std::{
    io::Write,
    sync::{Arc, atomic::AtomicBool},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::mpsc::UnboundedReceiver;

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
    // User,
    // AI,
}

#[derive(Clone)]
pub enum AppMode {
    Normal,
    Command,
    Editing,
    LeaderPending,
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
    pub agent_status: AgentStatus,
    pub spinner_index: usize,
    pub agent_bus: AgentBus,
    pub rx: UnboundedReceiver<RuntimeEvent>,
}

impl App {
    pub fn new(
        should_exit: Arc<AtomicBool>,
        agent_bus: AgentBus,
        rx: UnboundedReceiver<RuntimeEvent>,
    ) -> Self {
        let session_id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let logger = Logger::new(session_id);

        let (history_u, history_a) = logger.load_history();
        let mut combined_history = history_u.clone();
        combined_history.extend(history_a.clone());

        Self {
            rx,
            agent_bus,
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
            agent_status: AgentStatus::Waiting,
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

    fn history_mode(&mut self) {
        self.active_tab = (self.active_tab + self.tabs.len() - 1) % self.tabs.len();
    }

    pub fn handle_exit(&self) {
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
            self.mode = AppMode::Normal;
            return;
        }

        self.agent_status = AgentStatus::Thinking;

        let task = Task {
            id: uuid::Uuid::new_v4().to_string(),
            prompt: input,
        };

        let bus = self.agent_bus.clone();

        tokio::spawn(async move {
            let _ = bus.tx.send(AgentEvent::NewTask { task });
        });

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
    pub fn handle_events(&mut self) -> anyhow::Result<bool> {
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

        if self.agent_status == AgentStatus::Thinking {
            self.spinner_index += 1;
        }
        while let Ok(event) = self.rx.try_recv() {
            self.reduce(event);
        }
    }
    fn reduce(&mut self, event: RuntimeEvent) {
        match event {
            RuntimeEvent::Agent(agent_event) => {
                self.reduce_agent_event(agent_event);
            }

            RuntimeEvent::System(system_event) => {
                self.reduce_system_event(system_event);
            }
        }
    }
    fn reduce_agent_event(&mut self, event: AgentEvent) {
        match event {
            AgentEvent::Finished { result } => self.task_finished(result),

            AgentEvent::TaskEvent { task, event } => {
                self.apply_task_event(task, event);
            }

            _ => {}
        }
    }
    fn reduce_system_event(&mut self, event: SystemEvent) {
        match event {
            SystemEvent::TaskAdd { task_id } => {
                self.logs.push(format!("Task queued: {task_id}"));
            }
            SystemEvent::TaskCompleted { result } => {
                // print!("Task queued: {:?}", result);
                self.logs.push(format!("Task queued: {:?}", result));
            }
            _ => {}
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
            agent_status: &self.agent_status,
            spinner_index: self.spinner_index,
            cursor_visible: self.cursor_visible,
        }
    }
    fn apply_task_event(&mut self, task: Task, event: TaskEvent) {
        todo!("Hi")
    }
    fn task_finished(&mut self, result: TaskResult) {
        let summary = result.chat.unwrap_or_else(|| "done".to_string());
        let _ = self.logger.log_to_file("ai", &summary);
        self.ai_history.push(summary);
        self.mode = AppMode::Normal;
        self.agent_status = AgentStatus::Done;
    }
}
