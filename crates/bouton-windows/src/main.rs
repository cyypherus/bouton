#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]
#![allow(clippy::type_complexity, clippy::too_many_arguments)]

mod daemon;
mod injector;
mod keycode;
mod mappings;
mod server;
mod setup;
mod view;

use crate::daemon::{DaemonEvent, DaemonHandle, DaemonMsg};
use crate::keycode::KeyCode;
use crate::mappings::{DPadMapping, Mappings, StickId, StickMapping, TriggerMapping};
use crate::server::ServerEvent;
use crate::setup::UsbDevice;
use bouton_core::control::GamepadControl;
use haven::*;
use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpStatus {
    Idle,
    Opening,
    Ready,
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

    pub gp_devices: Vec<(String, String)>,
    pub gp_device_dd: DropdownState<String>,
    pub gp_refresh_btn: ButtonState,
    pub start_btn: ButtonState,
    pub stop_btn: ButtonState,
    pub gp_status: GpStatus,
    pub gp_detail: String,
    pub daemon: Option<DaemonHandle>,
    pub as_root: bool,
    pub pending_retry_as_root: bool,
    pub live_buttons: HashMap<GamepadControl, bool>,
    pub live_axes: HashMap<GamepadControl, i32>,
}

impl State {
    fn new(mappings: Mappings) -> Self {
        let addr_field = TextState::new(mappings.listen_addr.clone());
        let port_field = TextState::new(mappings.listen_port.to_string());
        let shared = Arc::new(Mutex::new(mappings.clone()));
        let (rebind_tx, _) = watch::channel((mappings.listen_addr.clone(), mappings.listen_port));
        let selected_busid = mappings.last_busid.clone().unwrap_or_default();
        let selected_device = mappings.last_device.clone().unwrap_or_default();

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

            gp_devices: Vec::new(),
            gp_device_dd: DropdownState {
                selected: selected_device,
                hovered: None,
                expanded: false,
            },
            gp_refresh_btn: ButtonState::default(),
            start_btn: ButtonState::default(),
            stop_btn: ButtonState::default(),
            gp_status: GpStatus::Idle,
            gp_detail: String::new(),
            daemon: None,
            as_root: false,
            pending_retry_as_root: false,
            live_buttons: HashMap::new(),
            live_axes: HashMap::new(),

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
            ListenSlot::Stick(stick, dir) => {
                self.mappings.sticks.get(&stick).map(|m| match dir {
                    StickDir::Up => m.up,
                    StickDir::Down => m.down,
                    StickDir::Left => m.left,
                    StickDir::Right => m.right,
                })
            }
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

fn on_server_event(state: &mut State, ev: ServerEvent) {
    match ev {
        ServerEvent::Listening(addr) => {
            state.status = ServerStatus::Listening;
            state.status_detail = addr.to_string();
            state.push_log(LogEntry::Info(format!("listening on {addr}")));
        }
        ServerEvent::BindFailed(err) => {
            state.status = ServerStatus::Failed;
            state.status_detail = err.clone();
            state.push_log(LogEntry::Error(format!("bind failed: {err}")));
        }
        ServerEvent::ClientConnected(addr) => {
            state.client = Some(addr);
            state.push_log(LogEntry::Info(format!("client {addr}")));
        }
        ServerEvent::KeyPressed(k) => {
            state.push_log(LogEntry::Key(format!("press {}", k.label())));
        }
        ServerEvent::KeyReleased(k) => {
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

fn on_daemon_event(state: &mut State, ev: DaemonEvent) {
    match ev {
        DaemonEvent::Msg(DaemonMsg::Device { .. }) => {}
        DaemonEvent::Msg(DaemonMsg::Opened) => {
            state.gp_status = GpStatus::Ready;
            state.gp_detail = state.gp_device_dd.selected.clone();
            state.push_log(LogEntry::Info(format!(
                "daemon opened {}",
                state.gp_device_dd.selected
            )));
        }
        DaemonEvent::Msg(DaemonMsg::PermissionDenied) => {
            state.push_log(LogEntry::Warn("permission denied".into()));
            if !state.as_root {
                state.pending_retry_as_root = true;
            } else {
                state.gp_status = GpStatus::Failed;
                state.gp_detail = "permission denied".into();
            }
        }
        DaemonEvent::Msg(DaemonMsg::OpenError { msg }) => {
            state.gp_status = GpStatus::Failed;
            state.gp_detail = msg.clone();
            state.push_log(LogEntry::Error(format!("daemon: {msg}")));
        }
        DaemonEvent::Msg(DaemonMsg::SendError { msg }) => {
            state.push_log(LogEntry::Warn(format!("daemon send: {msg}")));
        }
        DaemonEvent::Msg(DaemonMsg::Button { control, pressed }) => {
            state.live_buttons.insert(control, pressed);
        }
        DaemonEvent::Msg(DaemonMsg::Axis { control, value }) => {
            state.live_axes.insert(control, value);
        }
        DaemonEvent::Stderr(line) => {
            state.push_log(LogEntry::Warn(format!("wsl: {line}")));
        }
        DaemonEvent::Exited(code) => {
            state.daemon = None;
            state.live_buttons.clear();
            state.live_axes.clear();
            if !matches!(state.gp_status, GpStatus::Failed) && !state.pending_retry_as_root {
                state.gp_status = GpStatus::Idle;
                state.gp_detail = code
                    .map(|c| format!("exited {c}"))
                    .unwrap_or_else(|| "exited".into());
            }
            state.push_log(LogEntry::Info(format!("daemon exited {code:?}")));
        }
    }
}

pub fn start_daemon(state: &mut State, app: &mut AppState) {
    if state.daemon.is_some() {
        return;
    }
    let device = state.gp_device_dd.selected.clone();
    if device.is_empty() {
        state.push_log(LogEntry::Warn("no gamepad device selected".into()));
        return;
    }
    let server = format!("{}:{}", state.mappings.listen_addr, state.mappings.listen_port);
    state.gp_status = GpStatus::Opening;
    state.gp_detail = device.clone();
    state.push_log(LogEntry::Info(format!(
        "launching daemon {device} -> {server} (root: {})",
        state.as_root
    )));
    let cb = app.callback(on_daemon_event);
    let handle = daemon::run(device, server, state.as_root, move |ev| cb.send(ev));
    state.daemon = Some(handle);
}

pub fn stop_daemon(state: &mut State) {
    if let Some(mut h) = state.daemon.take() {
        h.stop();
    }
    state.as_root = false;
    state.pending_retry_as_root = false;
    state.gp_status = GpStatus::Idle;
    state.gp_detail.clear();
    state.live_buttons.clear();
    state.live_axes.clear();
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
            state.gp_device_dd.selected = state.mappings.last_device.clone().unwrap_or_default();
            state.sync_sliders_from_mappings();
            if let Ok(mut guard) = state.mappings_shared.lock() {
                *guard = state.mappings.clone();
            }
            let _ = state
                .rebind_tx
                .send((
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

        refresh_gp_devices(app);
    })
    .on_frame(|state, app| {
        if state.pending_retry_as_root && state.daemon.is_none() {
            state.pending_retry_as_root = false;
            state.as_root = true;
            start_daemon(state, app);
        }
    })
    .on_exit(|state, app| {
        if let Some(mut h) = state.daemon.take() {
            h.stop();
        }
        let m = state.mappings.clone();
        app.spawn(async move {
            mappings::save(m).await;
        });
    })
    .start();
}

pub fn refresh_gp_devices(app: &mut AppState) {
    let cb = app.callback(|s: &mut State, devs: Vec<(String, String)>| {
        s.gp_devices = devs;
        if s.gp_device_dd.selected.is_empty()
            && let Some((first, _)) = s.gp_devices.first()
        {
            s.gp_device_dd.selected = first.clone();
        }
    });
    app.spawn(async move {
        let devs = daemon::list_devices(false).await;
        cb.send(devs);
    });
}
