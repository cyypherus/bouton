use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};
use std::collections::VecDeque;

const MAX_LOG_LINES: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClientState {
    Waiting,
    Connected,
}

pub struct KeyInjectionState {
    pub last_key_code: Option<u32>,
    pub last_key_name: Option<String>,
    pub last_action: Option<String>,
    pub log: VecDeque<String>,
    pub client_state: ClientState,
    pub client_addr: Option<String>,
}

impl KeyInjectionState {
    pub fn new() -> Self {
        Self {
            last_key_code: None,
            last_key_name: None,
            last_action: None,
            log: VecDeque::new(),
            client_state: ClientState::Waiting,
            client_addr: None,
        }
    }

    pub fn log_key_injection(&mut self, key_name: String, action: String, key_code: u32) {
        self.last_key_code = Some(key_code);
        self.last_key_name = Some(key_name.clone());
        self.last_action = Some(action.clone());

        self.add_log(format!("{}: {}", key_name, action));
    }

    pub fn log_client_connected(&mut self, addr: String) {
        self.client_state = ClientState::Connected;
        self.client_addr = Some(addr.clone());
        self.add_log(format!("Client connected: {}", addr));
    }

    pub fn log_unbound(&mut self, control: String) {
        self.last_key_name = Some(control.clone());
        self.last_action = Some("unbound".to_string());
        self.last_key_code = None;
        self.add_log(format!("{}: unbound", control));
    }

    pub fn add_log(&mut self, msg: String) {
        self.log.push_front(msg);
        if self.log.len() > MAX_LOG_LINES {
            self.log.pop_back();
        }
    }
}

pub fn draw(f: &mut Frame, state: &KeyInjectionState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(4), Constraint::Min(0)])
        .split(f.area());

    draw_status(f, state, chunks[0]);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[1]);

    draw_last_key(f, state, main_chunks[0]);
    draw_log(f, state, main_chunks[1]);
}

fn draw_status(f: &mut Frame, state: &KeyInjectionState, area: Rect) {
    let client_status = match state.client_state {
        ClientState::Waiting => ("●", Color::Yellow),
        ClientState::Connected => ("●", Color::Green),
    };

    let client_text = if let Some(ref addr) = state.client_addr {
        format!("{} Client: {}", client_status.0, addr)
    } else {
        format!("{} Client: waiting for an event", client_status.0)
    };

    let status_text = vec![Line::from(vec![
        Span::styled(client_text, Style::default().fg(client_status.1)),
        Span::raw("  "),
        Span::styled("Windows Server", Style::default().fg(Color::Green)),
        Span::raw("  "),
        Span::styled(
            "Press Q or Esc to exit",
            Style::default().fg(Color::DarkGray),
        ),
    ])];

    let block = Block::default().borders(Borders::BOTTOM);
    let paragraph = Paragraph::new(status_text).block(block);
    f.render_widget(paragraph, area);
}

fn draw_last_key(f: &mut Frame, state: &KeyInjectionState, area: Rect) {
    let mut text = vec![];

    if let Some(key_name) = &state.last_key_name {
        text.push(Line::from(Span::styled(
            format!("{}", key_name),
            Style::default().fg(Color::Cyan),
        )));
        
        let action = state.last_action.as_ref().map(|s| s.as_str()).unwrap_or("unbound");
        let color = match action {
            "pressed" => Color::Green,
            "released" => Color::Red,
            "unbound" => Color::DarkGray,
            _ => Color::Gray,
        };
        text.push(Line::from(Span::styled(
            format!("{}", action),
            Style::default().fg(color),
        )));
        
        let code_text = if let Some(code) = state.last_key_code {
            format!("Code: 0x{:02X}", code)
        } else {
            "Code: unbound".to_string()
        };
        text.push(Line::from(Span::styled(
            code_text,
            Style::default().fg(Color::Gray),
        )));
    } else {
        text.push(Line::from(Span::styled(
            "No key pressed yet",
            Style::default().fg(Color::Gray),
        )));
    }

    let block = Block::default().title("Last Key").borders(Borders::ALL);
    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}

fn draw_log(f: &mut Frame, state: &KeyInjectionState, area: Rect) {
    let log_lines: Vec<Line> = state
        .log
        .iter()
        .map(|msg| Line::from(Span::raw(msg.clone())))
        .collect();

    let block = Block::default().title("Event Log").borders(Borders::ALL);
    let paragraph = Paragraph::new(log_lines).block(block);
    f.render_widget(paragraph, area);
}
