use bouton_core::GamepadEvent;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::collections::HashMap;
use std::collections::VecDeque;

const MAX_LOG_LINES: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Connecting,
    Connected,
    Error,
}

pub struct GamepadState {
    pub buttons: HashMap<u16, bool>,
    pub axes: HashMap<u16, i32>,
    pub log: VecDeque<String>,
    pub gamepad_state: ConnectionState,
    pub gamepad_error: Option<String>,
    pub server_state: ConnectionState,
    pub deadzone: i32,
}

impl GamepadState {
    pub fn new() -> Self {
        Self {
            buttons: HashMap::new(),
            axes: HashMap::new(),
            log: VecDeque::new(),
            gamepad_state: ConnectionState::Connecting,
            gamepad_error: None,
            server_state: ConnectionState::Connecting,
            deadzone: 0,
        }
    }

    pub fn update(&mut self, event: &GamepadEvent) {
        match event {
            GamepadEvent::Button { code, pressed } => {
                self.buttons.insert(*code, *pressed);
                let button_name = button_name(*code);
                let status = if *pressed { "PRESSED" } else { "RELEASED" };
                self.add_log(format!("Button {}: {} (code: 0x{:X})", button_name, status, code));
            }
            GamepadEvent::Axis { code, value } => {
                self.axes.insert(*code, *value);
                let axis_name = axis_name(*code);
                self.add_log(format!("Axis {}: {} (code: 0x{:X})", axis_name, value, code));
            }
        }
    }

    fn add_log(&mut self, msg: String) {
        self.log.push_back(msg);
        if self.log.len() > MAX_LOG_LINES {
            self.log.pop_front();
        }
    }
}

pub fn draw(f: &mut Frame, state: &GamepadState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(f.size());

    draw_status(f, state, chunks[0]);

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[1]);

    draw_buttons_and_axes(f, state, main_chunks[0]);
    draw_log(f, state, main_chunks[1]);
}

fn draw_status(f: &mut Frame, state: &GamepadState, area: Rect) {
    let gamepad_status = match state.gamepad_state {
        ConnectionState::Connecting => ("●", Color::Yellow),
        ConnectionState::Connected => ("●", Color::Green),
        ConnectionState::Error => ("⚠", Color::Red),
    };

    let server_status = match state.server_state {
        ConnectionState::Connecting => ("●", Color::Yellow),
        ConnectionState::Connected => ("●", Color::Green),
        ConnectionState::Error => ("⚠", Color::Red),
    };

    let gamepad_text = if let Some(ref err) = state.gamepad_error {
        format!("{} Gamepad: {}", gamepad_status.0, err)
    } else {
        format!("{} Gamepad", gamepad_status.0)
    };

    let status_text = vec![
        Line::from(vec![
            Span::styled(
                gamepad_text,
                Style::default().fg(gamepad_status.1),
            ),
            Span::raw(" "),
            Span::styled(
                format!("{} Server", server_status.0),
                Style::default().fg(server_status.1),
            ),
            Span::raw(if state.deadzone > 0 {
                format!("  (deadzone: {})", state.deadzone)
            } else {
                String::new()
            }),
        ]),
    ];

    let block = Block::default().borders(Borders::BOTTOM);
    let paragraph = Paragraph::new(status_text).block(block);
    f.render_widget(paragraph, area);
}

fn draw_buttons_and_axes(f: &mut Frame, state: &GamepadState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    draw_buttons(f, state, chunks[0]);
    draw_axes(f, state, chunks[1]);
}

fn draw_buttons(f: &mut Frame, state: &GamepadState, area: Rect) {
    let button_codes = vec![0x130, 0x131, 0x132, 0x133, 0x134, 0x135, 0x138, 0x139, 0x13A, 0x13B, 0x13D, 0x13C, 0x13E];
    let button_names = vec!["□ Square", "✕ Cross", "○ Circle", "△ Triangle", "L1", "R1", "Select", "Start", "L3", "R3", "Touch", "Aux1", "Aux2"];

    let mut text = vec![Line::from(Span::styled(
        "BUTTONS",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    ))];

    for (code, name) in button_codes.iter().zip(button_names.iter()) {
        let pressed = state.buttons.get(code).copied().unwrap_or(false);
        let style = if pressed {
            Style::default().fg(Color::Green).bg(Color::DarkGray)
        } else {
            Style::default().fg(Color::Gray)
        };
        text.push(Line::from(Span::styled(
            format!("  {} (0x{:X}): {}", name, code, if pressed { "●" } else { "○" }),
            style,
        )));
    }

    let block = Block::default()
        .title("Gamepad Buttons")
        .borders(Borders::ALL);
    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}

fn draw_axes(f: &mut Frame, state: &GamepadState, area: Rect) {
    let axis_codes = vec![0x00, 0x01, 0x02, 0x05, 0x03, 0x04, 0x10, 0x11];
    let axis_names = vec![
        "Left Stick X", 
        "Left Stick Y", 
        "Right Stick X",
        "Right Stick Y",
        "L2",
        "R2",
        "D-Pad X", 
        "D-Pad Y"
    ];

    let mut text = vec![Line::from(Span::styled(
        "AXES",
        Style::default()
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    ))];

    for (code, name) in axis_codes.iter().zip(axis_names.iter()) {
        let value = state.axes.get(code).copied().unwrap_or(0);
        text.push(Line::from(Span::styled(
            format!("  {}: {:6}", name, value),
            Style::default().fg(Color::Cyan),
        )));
    }

    let block = Block::default()
        .title("Axes")
        .borders(Borders::ALL);
    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}

fn draw_log(f: &mut Frame, state: &GamepadState, area: Rect) {
    let log_lines: Vec<Line> = state
        .log
        .iter()
        .map(|msg| Line::from(Span::raw(msg.clone())))
        .collect();

    let block = Block::default()
        .title("Event Log")
        .borders(Borders::ALL);
    let paragraph = Paragraph::new(log_lines).block(block);
    f.render_widget(paragraph, area);
}

fn button_name(code: u16) -> &'static str {
    match code {
        0x130 => "□ Square",
        0x131 => "✕ Cross",
        0x132 => "○ Circle",
        0x133 => "△ Triangle",
        0x134 => "L1",
        0x135 => "R1",
        0x136 => "L2",
        0x137 => "R2",
        0x138 => "Select",
        0x139 => "Start",
        0x13A => "L3",
        0x13B => "R3",
        0x13D => "Touch",
        0x13C => "Aux1",
        0x13E => "Aux2",
        _ => "Unknown",
    }
}

fn axis_name(code: u16) -> &'static str {
    match code {
        0x00 => "Left Stick X",
        0x01 => "Left Stick Y",
        0x02 => "Right Stick X",
        0x05 => "Right Stick Y",
        0x03 => "L2",
        0x04 => "R2",
        0x10 => "D-Pad X",
        0x11 => "D-Pad Y",
        _ => "Unknown",
    }
}
