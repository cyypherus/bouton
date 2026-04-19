use crate::injector;
use crate::keycode::KeyCode;
use crate::mappings::{DPadMapping, Mappings, StickId, StickMapping, TriggerMapping};
use bouton_core::{ControlEvent, KeyAction, control::GamepadControl};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::UdpSocket;

#[derive(Debug, Clone)]
pub enum ServerEvent {
    Listening(SocketAddr),
    BindFailed(String),
    ClientConnected(SocketAddr),
    KeyPressed(KeyCode),
    KeyReleased(KeyCode),
    Unbound(GamepadControl),
    Error(String),
}

#[derive(Default)]
struct State {
    stick_axes: HashMap<StickId, (u8, u8)>,
    stick_pressed: HashMap<StickId, (Option<KeyCode>, Option<KeyCode>)>,
    trigger_pressed: HashMap<GamepadControl, bool>,
    dpad_axes: (u8, u8),
    dpad_pressed: Option<KeyCode>,
    client: Option<SocketAddr>,
}

pub async fn run<E>(
    mappings: Arc<Mutex<Mappings>>,
    on_event: E,
    mut rebind: tokio::sync::watch::Receiver<(String, u16)>,
) where
    E: Fn(ServerEvent) + Send + Sync + 'static,
{
    'outer: loop {
        let (addr, port) = {
            let m = mappings.lock().unwrap();
            (m.listen_addr.clone(), m.listen_port)
        };
        let bind_addr: SocketAddr = match format!("{addr}:{port}").parse() {
            Ok(a) => a,
            Err(e) => {
                on_event(ServerEvent::BindFailed(e.to_string()));
                if rebind.changed().await.is_err() {
                    return;
                }
                continue;
            }
        };

        let socket = {
            let mut last_err: Option<String> = None;
            loop {
                match UdpSocket::bind(bind_addr).await {
                    Ok(s) => break s,
                    Err(e) => {
                        let msg = e.to_string();
                        if last_err.as_deref() != Some(msg.as_str()) {
                            on_event(ServerEvent::BindFailed(msg.clone()));
                            last_err = Some(msg);
                        }
                        tokio::select! {
                            _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {}
                            changed = rebind.changed() => {
                                if changed.is_err() { return; }
                                continue 'outer;
                            }
                        }
                    }
                }
            }
        };

        on_event(ServerEvent::Listening(bind_addr));

        let mut st = State::default();
        let mut buf = [0u8; 4096];

        loop {
            tokio::select! {
                res = socket.recv_from(&mut buf) => {
                    match res {
                        Ok((n, from)) => {
                            if st.client != Some(from) {
                                st.client = Some(from);
                                on_event(ServerEvent::ClientConnected(from));
                            }
                            if let Ok(ev) = bincode::deserialize::<ControlEvent>(&buf[..n]) {
                                let m = mappings.lock().unwrap().clone();
                                handle(ev, &m, &mut st, &on_event);
                            }
                        }
                        Err(e) => on_event(ServerEvent::Error(format!("recv: {e}"))),
                    }
                }
                changed = rebind.changed() => {
                    if changed.is_err() { return; }
                    break;
                }
            }
        }
    }
}

fn handle<E: Fn(ServerEvent)>(
    ev: ControlEvent,
    m: &Mappings,
    st: &mut State,
    on_event: &E,
) {
    match ev {
        ControlEvent::Button(b) => {
            if let Some(&key) = m.buttons.get(&b.control) {
                inject_key(key, b.action, on_event);
            } else {
                on_event(ServerEvent::Unbound(b.control));
            }
        }
        ControlEvent::Axis(a) => {
            let v = a.value as u8;
            match a.control {
                GamepadControl::LeftStickX | GamepadControl::LeftStickY => {
                    if let Some(cfg) = m.sticks.get(&StickId::Left) {
                        handle_stick(StickId::Left, a.control, v, cfg, st, on_event);
                    } else {
                        on_event(ServerEvent::Unbound(a.control));
                    }
                }
                GamepadControl::RightStickX | GamepadControl::RightStickY => {
                    if let Some(cfg) = m.sticks.get(&StickId::Right) {
                        handle_stick(StickId::Right, a.control, v, cfg, st, on_event);
                    } else {
                        on_event(ServerEvent::Unbound(a.control));
                    }
                }
                GamepadControl::L2 | GamepadControl::R2 => {
                    if let Some(cfg) = m.triggers.get(&a.control) {
                        handle_trigger(a.control, v, cfg, st, on_event);
                    } else {
                        on_event(ServerEvent::Unbound(a.control));
                    }
                }
                GamepadControl::DPadX | GamepadControl::DPadY => {
                    if let Some(cfg) = m.dpad {
                        handle_dpad(a.control, v, &cfg, st, on_event);
                    } else {
                        on_event(ServerEvent::Unbound(a.control));
                    }
                }
                _ => {}
            }
        }
    }
}

fn inject_key<E: Fn(ServerEvent)>(key: KeyCode, action: KeyAction, on_event: &E) {
    match injector::inject(key.vk(), action) {
        Ok(()) => on_event(match action {
            KeyAction::Press => ServerEvent::KeyPressed(key),
            KeyAction::Release => ServerEvent::KeyReleased(key),
        }),
        Err(e) => on_event(ServerEvent::Error(format!("inject {}: {e}", key.label()))),
    }
}

fn set_key<E: Fn(ServerEvent)>(
    slot: &mut Option<KeyCode>,
    new: Option<KeyCode>,
    on_event: &E,
) {
    if *slot == new {
        return;
    }
    if let Some(old) = *slot {
        inject_key(old, KeyAction::Release, on_event);
    }
    if let Some(k) = new {
        inject_key(k, KeyAction::Press, on_event);
    }
    *slot = new;
}

fn adaptive_in_deadzone(axis_diff: i16, perp_diff: i16, base: i16) -> bool {
    let max_range = 128i16;
    let max_dyn = 50i16;
    let ratio = perp_diff as f32 / max_range as f32;
    let dynamic = (max_dyn as f32 * ratio).ceil() as i16;
    let effective = base.max(dynamic);
    axis_diff < effective
}

fn handle_stick<E: Fn(ServerEvent)>(
    stick: StickId,
    control: GamepadControl,
    v: u8,
    cfg: &StickMapping,
    st: &mut State,
    on_event: &E,
) {
    let is_x = matches!(
        control,
        GamepadControl::LeftStickX | GamepadControl::RightStickX
    );
    let (mut x, mut y) = st.stick_axes.get(&stick).copied().unwrap_or((127, 127));
    if is_x {
        x = v;
    } else {
        y = v;
    }
    st.stick_axes.insert(stick, (x, y));

    let center = 127i16;
    let xd = (x as i16 - center).abs();
    let yd = (y as i16 - center).abs();
    let base = cfg.deadzone as i16;
    let x_in = adaptive_in_deadzone(xd, yd, base);
    let y_in = adaptive_in_deadzone(yd, xd, base);

    let (mut xk, mut yk) = st.stick_pressed.get(&stick).copied().unwrap_or((None, None));

    let new_x = if x_in {
        None
    } else if x > 127 {
        Some(cfg.right)
    } else {
        Some(cfg.left)
    };
    set_key(&mut xk, new_x, on_event);

    let new_y = if y_in {
        None
    } else if y > 127 {
        Some(cfg.down)
    } else {
        Some(cfg.up)
    };
    set_key(&mut yk, new_y, on_event);

    st.stick_pressed.insert(stick, (xk, yk));
}

fn handle_trigger<E: Fn(ServerEvent)>(
    control: GamepadControl,
    v: u8,
    cfg: &TriggerMapping,
    st: &mut State,
    on_event: &E,
) {
    let was = st.trigger_pressed.get(&control).copied().unwrap_or(false);
    let is = v > cfg.deadzone;
    if is != was {
        inject_key(
            cfg.key,
            if is { KeyAction::Press } else { KeyAction::Release },
            on_event,
        );
        st.trigger_pressed.insert(control, is);
    }
}

fn handle_dpad<E: Fn(ServerEvent)>(
    control: GamepadControl,
    v: u8,
    cfg: &DPadMapping,
    st: &mut State,
    on_event: &E,
) {
    let (mut x, mut y) = st.dpad_axes;
    if matches!(control, GamepadControl::DPadX) {
        x = v;
    } else {
        y = v;
    }
    st.dpad_axes = (x, y);

    let new = if x != 0 {
        Some(if x > 127 { cfg.left } else { cfg.right })
    } else if y != 0 {
        Some(if y > 127 { cfg.up } else { cfg.down })
    } else {
        None
    };
    set_key(&mut st.dpad_pressed, new, on_event);
}
