#![cfg_attr(
    all(target_os = "windows", not(debug_assertions)),
    windows_subsystem = "windows"
)]
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

mod injector;
mod keycode;
mod launch;
mod mappings;
mod server;
mod setup;
mod view;

use crate::keycode::KeyCode;
use crate::mappings::{DPadMapping, Mappings, StickId, StickMapping, TriggerMapping};
use crate::server::ServerEvent;
use crate::setup::UsbDevice;
use bouton_core::control::GamepadControl;
use haven::*;
use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio::sync::watch;

const MAX_LOG: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StickDir {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DPadDir {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone)]
pub enum LogEntry {
    Info(String),
    Key(String),
    Warn(String),
    Error(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServerStatus {
    Starting,
    Listening,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListenSlot {
    Button(GamepadControl),
    Stick(StickId, StickDir),
    Trigger(GamepadControl),
    DPad(DPadDir),
}

pub struct State {
    pub mappings: Mappings,
    pub mappings_shared: Arc<Mutex<Mappings>>,
    pub rebind_tx: watch::Sender<(String, u16)>,

    pub status: ServerStatus,
    pub status_detail: String,
    pub client: Option<SocketAddr>,
    pub log: VecDeque<LogEntry>,
    pub log_scroller: std::rc::Rc<std::cell::RefCell<ScrollerState>>,

    pub key_buttons: HashMap<ListenSlot, ButtonState>,
    pub listening: Option<ListenSlot>,

    pub stick_deadzone: HashMap<StickId, SliderState>,
    pub trigger_deadzone: HashMap<GamepadControl, SliderState>,

    pub addr_field: TextState,
    pub port_field: TextState,

    pub devices: Vec<UsbDevice>,
    pub device_dd: DropdownState<String>,
    pub refresh_btn: ButtonState,
    pub bind_btn: ButtonState,
    pub attach_btn: ButtonState,
    pub detach_btn: ButtonState,

    pub gp_device_field: TextState,
    pub launch_wsl_btn: ButtonState,
    pub sudo_toggle: ToggleState,
    pub launch_error: String,
    pub last_key: Option<(String, Instant)>,
}

impl State {
    fn new(mappings: Mappings) -> Self {
        let addr_field = TextState::new(mappings.listen_addr.clone());
        let port_field = TextState::new(mappings.listen_port.to_string());
        let shared = Arc::new(Mutex::new(mappings.clone()));
        let (rebind_tx, _) = watch::channel((mappings.listen_addr.clone(), mappings.listen_port));
        let selected_busid = mappings.last_busid.clone().unwrap_or_default();
        let last_device = mappings.last_device.clone().unwrap_or_default();

        let mut s = Self {
            key_buttons: HashMap::new(),
            listening: None,
            stick_deadzone: HashMap::new(),
            trigger_deadzone: HashMap::new(),

            addr_field,
            port_field,

            devices: Vec::new(),
            device_dd: DropdownState {
                selected: selected_busid,
                hovered: None,
                expanded: false,
            },
            refresh_btn: ButtonState::default(),
            bind_btn: ButtonState::default(),
            attach_btn: ButtonState::default(),
            detach_btn: ButtonState::default(),

            gp_device_field: TextState::new(last_device),
            launch_wsl_btn: ButtonState::default(),
            sudo_toggle: if mappings.sudo {
                ToggleState::on()
            } else {
                ToggleState::off()
            },
            launch_error: String::new(),
            last_key: None,

            status: ServerStatus::Starting,
            status_detail: String::new(),
            client: None,
            log: VecDeque::new(),
            log_scroller: std::rc::Rc::new(std::cell::RefCell::new(ScrollerState::default())),

            mappings,
            mappings_shared: shared,
            rebind_tx,
        };
        s.sync_sliders_from_mappings();
        s
    }

    pub fn sync_sliders_from_mappings(&mut self) {
        for stick in [StickId::Left, StickId::Right] {
            let cfg = self
                .mappings
                .sticks
                .get(&stick)
                .copied()
                .unwrap_or(default_stick());
            let value = cfg.deadzone as f32 / 128.0;
            self.stick_deadzone
                .entry(stick)
                .and_modify(|s| s.value = value)
                .or_insert(SliderState {
                    value,
                    ..Default::default()
                });
        }
        for t in [GamepadControl::L2, GamepadControl::R2] {
            let cfg = self
                .mappings
                .triggers
                .get(&t)
                .copied()
                .unwrap_or(TriggerMapping {
                    key: KeyCode::C,
                    deadzone: 30,
                });
            let value = cfg.deadzone as f32 / 255.0;
            self.trigger_deadzone
                .entry(t)
                .and_modify(|s| s.value = value)
                .or_insert(SliderState {
                    value,
                    ..Default::default()
                });
        }
    }

    pub fn apply_captured_key(&mut self, key: KeyCode) {
        let Some(slot) = self.listening.take() else {
            return;
        };
        match slot {
            ListenSlot::Button(c) => {
                self.mappings.buttons.insert(c, key);
            }
            ListenSlot::Stick(stick, dir) => {
                let entry = self
                    .mappings
                    .sticks
                    .entry(stick)
                    .or_insert_with(default_stick);
                match dir {
                    StickDir::Up => entry.up = key,
                    StickDir::Down => entry.down = key,
                    StickDir::Left => entry.left = key,
                    StickDir::Right => entry.right = key,
                }
            }
            ListenSlot::Trigger(c) => {
                self.mappings
                    .triggers
                    .entry(c)
                    .and_modify(|t| t.key = key)
                    .or_insert(TriggerMapping { key, deadzone: 30 });
            }
            ListenSlot::DPad(dir) => {
                let d = self.mappings.dpad.get_or_insert(DPadMapping {
                    up: KeyCode::Up,
                    down: KeyCode::Down,
                    left: KeyCode::Left,
                    right: KeyCode::Right,
                });
                match dir {
                    DPadDir::Up => d.up = key,
                    DPadDir::Down => d.down = key,
                    DPadDir::Left => d.left = key,
                    DPadDir::Right => d.right = key,
                }
            }
        }
    }

    pub fn slot_current_key(&self, slot: ListenSlot) -> Option<KeyCode> {
        match slot {
            ListenSlot::Button(c) => self.mappings.buttons.get(&c).copied(),
            ListenSlot::Stick(stick, dir) => self.mappings.sticks.get(&stick).map(|m| match dir {
                StickDir::Up => m.up,
                StickDir::Down => m.down,
                StickDir::Left => m.left,
                StickDir::Right => m.right,
            }),
            ListenSlot::Trigger(c) => self.mappings.triggers.get(&c).map(|t| t.key),
            ListenSlot::DPad(dir) => self.mappings.dpad.map(|d| match dir {
                DPadDir::Up => d.up,
                DPadDir::Down => d.down,
                DPadDir::Left => d.left,
                DPadDir::Right => d.right,
            }),
        }
    }

    pub fn push_log(&mut self, entry: LogEntry) {
        self.log.push_back(entry);
        while self.log.len() > MAX_LOG {
            self.log.pop_front();
        }
    }

    pub fn persist(&self, app: &mut AppState) {
        if let Ok(mut guard) = self.mappings_shared.lock() {
            *guard = self.mappings.clone();
        }
        let m = self.mappings.clone();
        app.spawn(async move {
            mappings::save(m).await;
        });
    }
}

fn default_stick() -> StickMapping {
    StickMapping {
        deadzone: 20,
        up: KeyCode::W,
        down: KeyCode::S,
        left: KeyCode::A,
        right: KeyCode::D,
    }
}

pub const BUTTON_CONTROLS: &[GamepadControl] = &[
    GamepadControl::Square,
    GamepadControl::Cross,
    GamepadControl::Circle,
    GamepadControl::Triangle,
    GamepadControl::L1,
    GamepadControl::R1,
    GamepadControl::L3,
    GamepadControl::R3,
    GamepadControl::Select,
    GamepadControl::Start,
    GamepadControl::Touch,
    GamepadControl::Aux1,
    GamepadControl::Aux2,
];

fn is_firewall_error(err: &str) -> bool {
    let lower = err.to_ascii_lowercase();
    lower.contains("only one usage of each socket address")
        || lower.contains("permission denied")
        || lower.contains("access is denied")
        || lower.contains("access denied")
        || lower.contains("forbidden")
        || lower.contains("(os error 10013)")
        || lower.contains("(os error 10048)")
}

fn on_server_event(state: &mut State, ev: ServerEvent) {
    match ev {
        ServerEvent::Listening(addr) => {
            state.status = ServerStatus::Listening;
            state.status_detail = addr.to_string();
            state.push_log(LogEntry::Info(format!("listening on {addr}")));
        }
        ServerEvent::BindFailed(err) => {
            state.status = ServerStatus::Failed;
            let hint = if is_firewall_error(&err) {
                " (Windows Firewall may be blocking — click Allow if prompted; retrying…)"
            } else {
                " (retrying…)"
            };
            state.status_detail = format!("{err}{hint}");
            state.push_log(LogEntry::Error(format!("bind failed: {err}{hint}")));
        }
        ServerEvent::ClientConnected(addr) => {
            state.client = Some(addr);
            state.push_log(LogEntry::Info(format!("client {addr}")));
        }
        ServerEvent::KeyPressed(k) => {
            state.last_key = Some((format!("press {}", k.label()), Instant::now()));
            state.push_log(LogEntry::Key(format!("press {}", k.label())));
        }
        ServerEvent::KeyReleased(k) => {
            state.last_key = Some((format!("release {}", k.label()), Instant::now()));
            state.push_log(LogEntry::Key(format!("release {}", k.label())));
        }
        ServerEvent::Unbound(c) => {
            state.push_log(LogEntry::Warn(format!("unbound: {c}")));
        }
        ServerEvent::Error(e) => {
            state.push_log(LogEntry::Error(e));
        }
    }
}

pub fn launch_wsl_client(state: &mut State, app: &mut AppState) {
    let device = state.gp_device_field.text.trim().to_string();
    if device.is_empty() {
        state.launch_error =
            "Enter a WSL device path (e.g. /dev/input/event8) before launching.".into();
        state.push_log(LogEntry::Warn(state.launch_error.clone()));
        return;
    }
    state.mappings.last_device = Some(device.clone());
    let server = format!(
        "{}:{}",
        state.mappings.listen_addr, state.mappings.listen_port
    );
    let sudo = state.mappings.sudo;
    let dev_for_log = device.clone();
    let server_for_log = server.clone();
    state.push_log(LogEntry::Info(format!(
        "launching WSL client: {}bouton-linux --run {dev_for_log} {server_for_log}",
        if sudo { "sudo " } else { "" }
    )));
    let cb = app.callback(|s: &mut State, res: Result<(), String>| match res {
        Ok(()) => s.launch_error.clear(),
        Err(e) => {
            s.launch_error = e.clone();
            s.push_log(LogEntry::Error(format!("launch: {e}")));
        }
    });
    app.spawn(async move {
        let res = tokio::task::spawn_blocking(move || {
            launch::launch_wsl_client(&device, &server, sudo)
        })
        .await
        .unwrap_or_else(|e| Err(format!("task panic: {e}")));
        cb.send(res);
    });
    state.persist(app);
}

fn main() {
    App::builder(
        State::new(Mappings::default()),
        Window::new("main", view::root)
            .title("bouton")
            .inner_size(960, 770),
    )
    .on_start(|state, app| {
        let event_cb = app.callback(on_server_event);
        let shared = state.mappings_shared.clone();
        let rebind = state.rebind_tx.subscribe();
        app.spawn(async move {
            server::run(shared, move |ev| event_cb.send(ev), rebind).await;
        });

        let loaded = app.callback(|state: &mut State, m: Mappings| {
            state.mappings = m;
            state.addr_field = TextState::new(state.mappings.listen_addr.clone());
            state.port_field = TextState::new(state.mappings.listen_port.to_string());
            state.device_dd.selected = state.mappings.last_busid.clone().unwrap_or_default();
            state.gp_device_field =
                TextState::new(state.mappings.last_device.clone().unwrap_or_default());
            state.sudo_toggle = if state.mappings.sudo {
                ToggleState::on()
            } else {
                ToggleState::off()
            };
            state.sync_sliders_from_mappings();
            if let Ok(mut guard) = state.mappings_shared.lock() {
                *guard = state.mappings.clone();
            }
            let _ = state.rebind_tx.send((
                state.mappings.listen_addr.clone(),
                state.mappings.listen_port,
            ));
        });
        app.spawn(async move {
            loaded.send(mappings::load().await);
        });

        let devices_cb = app.callback(|state: &mut State, devs: Vec<UsbDevice>| {
            state.devices = devs;
            if state.device_dd.selected.is_empty()
                && let Some(first) = state.devices.first()
            {
                state.device_dd.selected = first.busid.clone();
            }
        });
        app.spawn(async move {
            let devs = tokio::task::spawn_blocking(setup::list_devices)
                .await
                .ok()
                .and_then(|r| r.ok())
                .unwrap_or_default();
            devices_cb.send(devs);
        });
    })
    .on_exit(|state, app| {
        let m = state.mappings.clone();
        app.spawn(async move {
            mappings::save(m).await;
        });
    })
    .start();
}
