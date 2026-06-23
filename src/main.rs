pub use anyhow::Error;
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
mod app;
mod logger;
mod ui;
use app::App;
use ui::{UiState, render_ui};

use crate::app::Runner;

pub fn print_user(input: &str) {
    println!("{}: {}", "You".blue().bold(), input);
}

pub fn print_ai(answer: &str) {
    println!("{}: {}", "AI".green().bold(), answer);
}

pub fn print_system(msg: &str) {
    println!("{} {}", "::".yellow(), msg.italic());
}

// #[derive(Clone)]
// pub struct UiState<'a> {
//     pub messages: &'a [Message],
//     pub mode: &'a AppMode,
//     pub history: &'a [String],
//     pub logs: &'a [String],
//     pub tabs: &'a [String],
//     pub active_tab: usize,
//     pub input_buffer: &'a str,
//     pub agent_mode: &'a AgentMode,
//     pub spinner_index: usize,
//     pub cursor_visible: bool,
//     pub user_history: &'a [String],
//     pub ai_history: &'a [String],
// }

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

// pub fn render_ui(f: &mut Frame, app: &mut App) {
//     let chunks = Layout::default()
//         .direction(Direction::Vertical)
//         .constraints([
//             Constraint::Length(3),
//             Constraint::Min(1),
//             Constraint::Length(3),
//             Constraint::Length(3),
//         ])
//         .split(f.area());

//     let content_chunks = Layout::default()
//         .direction(Direction::Horizontal)
//         .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
//         .split(chunks[1]);

//     app.ai_area = content_chunks[0];
//     app.user_area = content_chunks[1];
//     let (ai_area, user_area) = (app.ai_area, app.user_area);
//     app.scroll_all_to_bottom(content_chunks[0].width);
//     render_conversation(f, app, ai_area);
//     render_history(f, app, user_area);

//     {
//         let state = app.get_ui_data();
//         render_tabs(f, &state, chunks[0]);
//         f.render_widget(create_hint_widget(state.clone()), chunks[2]);
//         f.render_widget(create_input_widget(state), chunks[3]);
//     }
// }

// fn render_tabs(f: &mut Frame, state: &UiState, area: Rect) {
//     f.render_widget(
//         Tabs::new(state.tabs.iter().map(|s| s.as_str()))
//             .select(state.active_tab)
//             .block(Block::default().borders(Borders::ALL).title(" Gideon ")),
//         area,
//     );
// }

// fn render_conversation(f: &mut Frame, app: &mut App, area: Rect) {
//     use textwrap::{Options, wrap};
//     let width = area.width.saturating_sub(4) as usize;

//     // 1. Generate the items (This is the same logic as your rendering)
//     let msg_items: Vec<ListItem> = app
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

//     // 2. Initial scroll logic (Done AFTER generating items so we know the count)
//     if !app.is_initialized {
//         let total_lines = msg_items.len();
//         if total_lines > 0 {
//             // Set the offset to show the last possible line
//             // We subtract the area.height to show the end of the list,
//             // otherwise it just jumps to the very first line of the last item.
//             let view_height = area.height.saturating_sub(2) as usize;
//             *app.ai_list_state.offset_mut() = total_lines.saturating_sub(view_height);
//         }
//         app.is_initialized = true;
//     }

//     // 3. Render
//     let list = List::new(msg_items).block(
//         Block::default()
//             .borders(Borders::ALL)
//             .title(" AI Responses "),
//     );

//     f.render_stateful_widget(list, area, &mut app.ai_list_state);
// }
// fn render_history(f: &mut Frame, app: &mut App, area: Rect) {
//     use textwrap::{Options, wrap};
//     let width = area.width.saturating_sub(4) as usize;

//     let hist_items: Vec<ListItem> = app
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

//     // Initial scroll logic for the history column
//     if !app.is_initialized {
//         let total_lines = hist_items.len();
//         if total_lines > 0 {
//             let view_height = area.height.saturating_sub(2) as usize;
//             *app.user_list_state.offset_mut() = total_lines.saturating_sub(view_height);
//         }
//     }

//     let list = List::new(hist_items)
//         .block(
//             Block::default()
//                 .borders(Borders::ALL)
//                 .title(" Your Prompts "),
//         )
//         .highlight_symbol(">> ");

//     f.render_stateful_widget(list, area, &mut app.user_list_state);
// }
// fn create_hint_widget(state: UiState<'_>) -> Paragraph<'_> {
//     let normal = ":q to quit | I to Edit | Enter to Send";
//     let hint_text: &str = match state.mode {
//         AppMode::Normal => normal,
//         AppMode::Editing => "Esc to Normal | Ctrl+S to Save",
//         _ => "hi",
//     };

//     Paragraph::new(hint_text)
//         .block(Block::default().borders(Borders::ALL).title(" Hints "))
//         .style(
//             Style::default()
//                 .fg(Color::Yellow)
//                 .add_modifier(Modifier::DIM),
//         )
//         .alignment(Alignment::Center)
// }
// fn create_input_widget<'a>(state: UiState) -> Paragraph<'a> {
//     let spinner_chars = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
//     let prefix = if state.agent_mode == &AgentMode::Thinking {
//         spinner_chars[state.spinner_index % spinner_chars.len()].to_string()
//     } else if !state.input_buffer.is_empty()
//         && matches!(
//             state.input_buffer.chars().next().unwrap(),
//             ':' | '!' | '/' | '.' | '#' | '@'
//         )
//     {
//         state.input_buffer.chars().next().unwrap().to_string()
//     } else {
//         ">".to_string()
//     };
//     let mut spans = vec![Span::styled(
//         format!("{} ", prefix),
//         Style::default().fg(Color::Yellow).bold(),
//     )];

//     let content = if state.agent_mode == &AgentMode::Thinking {
//         "Thinking...".to_string()
//     } else {
//         state.input_buffer.to_string()
//     };

//     spans.push(Span::styled(content, Style::default().fg(Color::White)));
//     if state.agent_mode != &AgentMode::Thinking && state.cursor_visible {
//         spans.push(Span::styled("_", Style::default().fg(Color::Yellow).bold()));
//     }
//     let line = Line::from(spans);
//     Paragraph::new(line).block(Block::default().borders(Borders::ALL).title(" Input "))
// }

// pub async fn run_agent_loop(user_input: String) -> anyhow::Result<()> {
//     let command = prompt_ollama_for_json(&user_input).await?;
//     match command {
//         AgentCommand::WriteFile { path, content } => {
//             let target = std::path::PathBuf::from("./allowed_dir/output.txt");
//             if let Some(parent) = target.parent()
//                 && let Err(e) = std::fs::create_dir_all(parent)
//             {
//                 eprintln!("Failed to create directory: {}", e);
//                 return Err(e.into());
//             }

//             match std::fs::write(&target, content) {
//                 Ok(_) => println!("Successfully wrote to {:?}", target),
//                 Err(e) => eprintln!("Failed to write file: {}", e),
//             }
//         }
//         AgentCommand::Chat { message } => {
//             println!("AI: {}", message);
//         }
//         _ => {
//             todo!("hi run_agent_loop");
//         }
//     }
//     Ok(())
// }

async fn prompt_ollama_for_json(user_input: &str) -> anyhow::Result<AgentCommand> {
    let client = reqwest::Client::new();

    let system_prompt = r#"You are an AI assistant with file system access.
        If the user wants to save, create, or update a file, return: 
        {"type": "WriteFile", "data": {"path": "./allowed_dir/output.txt", "content": "FILE_CONTENT"}}
        Otherwise, return:
        {"type": "Chat", "data": {"message": "Your response here"}}"#;

    let payload = serde_json::json!({
        "model": "qwen3:8b",
        "system": system_prompt,
        "prompt": user_input,
        "stream": false,
        "format": "json"
    });

    let res = client
        .post("http://localhost:11434/api/generate")
        .json(&payload)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?;

    // Extract the response field from Ollama and deserialize into our enum
    let response_text = res["response"].as_str().unwrap_or("{}");
    let cmd: AgentCommand = serde_json::from_str(response_text)?;

    Ok(cmd)
}

async fn prompt_ollama(user_input: &str) -> anyhow::Result<AgentCommand> {
    use reqwest::Client;
    use serde_json::json;

    let client = Client::new();
    let url = "http://localhost:11434/api/generate";

    // 1. Define the system/format instructions
    let system_instructions = r#"
        You are a JSON-only API. Respond ONLY with a valid JSON object matching one of these:
        {"type": "WriteFile", "data": {"path": "...", "content": "..."}}
        {"type": "ReadFile", "data": {"path": "..."}}
        {"type": "Chat", "data": {"message": "..."}}
    "#;

    let payload = json!({
        "model": "qwen3:8b",
        "prompt": format!("{}\nUser Request: {}", system_instructions, user_input),
        "stream": false,
        "format": "json" // Tells Ollama to prioritize JSON structure
    });

    // 2. Send request
    let res = client
        .post(url)
        .json(&payload)
        .send()
        .await?
        .json::<OllamaResponse>()
        .await?;

    // 3. Parse the response string back into your enum
    // We expect the model's 'response' field to be the JSON string
    let command: AgentCommand = serde_json::from_str(&res.response)?;

    Ok(command)
}

#[derive(Deserialize, Debug)]
struct OllamaResponse {
    done: bool,
    model: String,
    response: String, // This is the string we will parse into AgentCommand
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
enum AgentCommand {
    WriteFile { path: String, content: String },
    ReadFile { path: String },
    Chat { message: String },
}

static WRITE_PROMPT: &str = r#"You are an AI assistant with file system access.
        If the user wants to save, create, or update a file, return: 
        {"type": "WriteFile", "data": {"path": "./allowed_dir/output.txt", "content": "FILE_CONTENT"}}
        Otherwise, return:
        {"type": "Chat", "data": {"message": "Your response here"}}"#;

static SYSTEM_PROMPT: &str = r#"
You are an intelligent file system assistant. You must always respond with a valid JSON object that matches one of these structures:

1. To write a file:
   {"type": "WriteFile", "data": {"path": "...", "content": "..."}}

2. To read a file:
   {"type": "ReadFile", "data": {"path": "..."}}

3. To communicate:
   {"type": "Chat", "data": {"message": "..."}}

Rules:
- Do not include any text outside the JSON object.
- Ensure all paths are strings.
- Escape newlines and quotes correctly within the "content" or "message" fields.
"#;
