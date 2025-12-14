use bouton_core::{ControlEvent, KeyAction, control::GamepadControl};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
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
    pub buttons: HashMap<GamepadControl, bool>,
    pub axes: HashMap<GamepadControl, i32>,
    pub log: VecDeque<String>,
    pub gamepad_state: ConnectionState,
    pub gamepad_error: Option<String>,
    pub server_state: ConnectionState,
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
        }
    }

    pub fn update(&mut self, event: &ControlEvent) {
        match event {
            ControlEvent::Button(btn) => {
                self.buttons.insert(btn.control, matches!(btn.action, KeyAction::Press));
                let status = match btn.action {
                    KeyAction::Press => "PRESSED",
                    KeyAction::Release => "RELEASED",
                };
                self.add_log(format!(
                    "Button {}: {}",
                    btn.control, status
                ));
            }
            ControlEvent::Axis(axis) => {
                self.axes.insert(axis.control, axis.value);
                self.add_log(format!(
                    "Axis {}: {}",
                    axis.control, axis.value
                ));
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
        .constraints([Constraint::Length(4), Constraint::Min(0)])
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

    let status_text = vec![Line::from(vec![
        Span::styled(gamepad_text, Style::default().fg(gamepad_status.1)),
        Span::raw(" "),
        Span::styled(
            format!("{} Server", server_status.0),
            Style::default().fg(server_status.1),
        ),
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

fn draw_buttons_and_axes(f: &mut Frame, state: &GamepadState, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    draw_buttons(f, state, chunks[0]);
    draw_axes(f, state, chunks[1]);
}

fn draw_buttons(f: &mut Frame, state: &GamepadState, area: Rect) {
    let buttons = vec![
        GamepadControl::Square,
        GamepadControl::Cross,
        GamepadControl::Circle,
        GamepadControl::Triangle,
        GamepadControl::L1,
        GamepadControl::R1,
        GamepadControl::Select,
        GamepadControl::Start,
        GamepadControl::L3,
        GamepadControl::R3,
        GamepadControl::Touch,
        GamepadControl::Aux1,
        GamepadControl::Aux2,
    ];

    let mut text = vec![];

    for control in buttons {
        let pressed = state.buttons.get(&control).copied().unwrap_or(false);
        let style = if pressed {
            Style::default().fg(Color::Green).bg(Color::DarkGray)
        } else {
            Style::default().fg(Color::Gray)
        };
        text.push(Line::from(Span::styled(
            format!(
                "  {}: {}",
                control,
                if pressed { "●" } else { "○" }
            ),
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
    let axes = vec![
        GamepadControl::LeftStickX,
        GamepadControl::LeftStickY,
        GamepadControl::RightStickX,
        GamepadControl::RightStickY,
        GamepadControl::L2,
        GamepadControl::R2,
        GamepadControl::DPadX,
        GamepadControl::DPadY,
    ];

    let mut text = vec![];

    for control in axes {
        let value = state.axes.get(&control).copied().unwrap_or(0);
        text.push(Line::from(Span::styled(
            format!("  {}: {:6}", control, value),
            Style::default().fg(Color::Cyan),
        )));
    }

    let block = Block::default().title("Axes").borders(Borders::ALL);
    let paragraph = Paragraph::new(text).block(block);
    f.render_widget(paragraph, area);
}

fn draw_log(f: &mut Frame, state: &GamepadState, area: Rect) {
    let log_lines: Vec<Line> = state
        .log
        .iter()
        .map(|msg| Line::from(Span::raw(msg.clone())))
        .collect();

    let block = Block::default().title("Event Log").borders(Borders::ALL);
    let paragraph = Paragraph::new(log_lines).block(block);
    f.render_widget(paragraph, area);
}
