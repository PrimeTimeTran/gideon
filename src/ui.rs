use colored::*;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
};

use crate::{
    agent::AgentStatus,
    app::{App, AppMode, Message},
};

#[derive(Clone)]
pub struct UiState<'a> {
    pub messages: &'a [Message],
    pub mode: &'a AppMode,
    pub history: &'a [String],
    pub logs: &'a [String],
    pub tabs: &'a [String],
    pub active_tab: usize,
    pub input_buffer: &'a str,
    pub agent_status: &'a AgentStatus,
    pub spinner_index: usize,
    pub cursor_visible: bool,
    pub user_history: &'a [String],
    pub ai_history: &'a [String],
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
    app.scroll_all_to_bottom(content_chunks[0].width);
    render_conversation(f, app, ai_area);
    render_history(f, app, user_area);

    {
        let state: UiState<'_> = app.get_ui_data();
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

    if !app.is_initialized {
        let total_lines = msg_items.len();
        if total_lines > 0 {
            let view_height = area.height.saturating_sub(2) as usize;
            *app.ai_list_state.offset_mut() = total_lines.saturating_sub(view_height);
        }
        app.is_initialized = true;
    }

    // 3. Render
    let list = List::new(msg_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" AI Responses "),
    );

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

    if !app.is_initialized {
        let total_lines = hist_items.len();
        if total_lines > 0 {
            let view_height = area.height.saturating_sub(2) as usize;
            *app.user_list_state.offset_mut() = total_lines.saturating_sub(view_height);
        }
    }

    let list = List::new(hist_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Your Prompts "),
        )
        .highlight_symbol(">> ");

    f.render_stateful_widget(list, area, &mut app.user_list_state);
}
fn create_hint_widget(state: UiState<'_>) -> Paragraph<'_> {
    let normal = ":q to quit | I to Edit | Enter to Send";
    let hint_text: &str = match state.mode {
        AppMode::Normal => normal,
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
    let prefix = if state.agent_status == &AgentStatus::Thinking {
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

    let content = if state.agent_status == &AgentStatus::Thinking {
        "Thinking...".to_string()
    } else {
        state.input_buffer.to_string()
    };

    spans.push(Span::styled(content, Style::default().fg(Color::White)));
    if state.agent_status != &AgentStatus::Thinking && state.cursor_visible {
        spans.push(Span::styled("_", Style::default().fg(Color::Yellow).bold()));
    }
    let line = Line::from(spans);
    Paragraph::new(line).block(Block::default().borders(Borders::ALL).title(" Input "))
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
