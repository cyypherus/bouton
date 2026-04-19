use crate::keycode::KeyCode;
use crate::mappings::{StickId, TriggerMapping};
use crate::{
    BUTTON_CONTROLS, DPadDir, GpStatus, ListenSlot, LogEntry, ServerStatus, State, StickDir,
    default_stick, refresh_gp_devices, setup, start_daemon, stop_daemon,
};
use bouton_core::control::GamepadControl;
use haven::*;

const BG: Color = Color::from_rgb8(24, 24, 27);
const PANEL: Color = Color::from_rgb8(32, 32, 36);
const PANEL_HI: Color = Color::from_rgb8(44, 44, 50);
const BORDER: Color = Color::from_rgb8(56, 56, 62);
const FG: Color = Color::from_rgb8(230, 230, 234);
const FG_DIM: Color = Color::from_rgb8(150, 150, 156);
const ACCENT: Color = Color::from_rgb8(113, 70, 232);
const GREEN: Color = Color::from_rgb8(80, 200, 120);
const YELLOW: Color = Color::from_rgb8(230, 190, 80);
const RED: Color = Color::from_rgb8(230, 90, 90);
const KEY_COLOR: Color = Color::from_rgb8(140, 190, 255);

const LABEL_W: f32 = 65.;
const KEY_W: f32 = 130.;
const VALUE_W: f32 = 30.;
const SLIDER_W: f32 = KEY_W - VALUE_W - 5.;
const ROW_H: f32 = 24.;
const LABEL_SIZE: u32 = 12;

pub fn root<'a>(s: &'a State, app: &mut AppState) -> Layout<'a, View<State>, AppCtx> {
    stack(vec![
        rect(id!()).fill(BG).build(app.ctx()),
        column(vec![
            header(s, app).height(50.),
            row_spaced(
                10.,
                vec![
                    mappings_panel(s, app).width(500.),
                    column_spaced(
                        10.,
                        vec![
                            setup_panel(s, app).height(200.),
                            gamepad_panel(s, app),
                            log_panel(s, app).height(180.),
                        ],
                    ),
                ],
            )
            .pad(10.),
        ]),
        if s.listening.is_some() {
            key_listener(app)
        } else {
            empty()
        },
    ])
}

fn key_listener<'a>(app: &mut AppState) -> Layout<'a, View<State>, AppCtx> {
    rect(id!())
        .fill(Color::TRANSPARENT)
        .view()
        .on_key(|s: &mut State, app, key| {
            if s.listening.is_none() {
                return;
            }
            if key == Key::Named(NamedKey::Escape) {
                s.listening = None;
                return;
            }
            let Some(kc) = KeyCode::from_key(&key) else {
                return;
            };
            s.apply_captured_key(kc);
            s.persist(app);
        })
        .finish(app.ctx())
}

fn header<'a>(s: &'a State, app: &mut AppState) -> Layout<'a, View<State>, AppCtx> {
    let (server_color, server_text) = match s.status {
        ServerStatus::Starting => (YELLOW, "starting".to_string()),
        ServerStatus::Listening => (GREEN, format!("listening on {}", s.status_detail)),
        ServerStatus::Failed => (RED, format!("bind failed: {}", s.status_detail)),
    };
    let (gp_color, gp_text) = match s.gp_status {
        GpStatus::Idle => (FG_DIM, "gamepad idle".to_string()),
        GpStatus::Opening => (YELLOW, format!("gamepad opening {}", s.gp_detail)),
        GpStatus::Ready => (GREEN, format!("gamepad {}", s.gp_detail)),
        GpStatus::Failed => (RED, format!("gamepad failed: {}", s.gp_detail)),
    };
    let (client_color, client_text) = match s.client {
        Some(addr) => (GREEN, format!("client {addr}")),
        None => (FG_DIM, "no client".to_string()),
    };
    stack(vec![
        rect(id!()).fill(PANEL).build(app.ctx()),
        row_spaced(
            20.,
            vec![
                text(id!(), "bouton")
                    .fill(FG)
                    .font_weight(FontWeight::BOLD)
                    .font_size(20)
                    .build(app.ctx())
                    .width(90.),
                dot_label(app, server_color, server_text),
                dot_label(app, gp_color, gp_text),
                dot_label(app, client_color, client_text),
                space(),
            ],
        )
        .pad_x(16.),
    ])
}

fn dot_label<'a>(
    app: &mut AppState,
    color: Color,
    label: String,
) -> Layout<'a, View<State>, AppCtx> {
    row_spaced(
        6.,
        vec![
            circle(id!(str_hash(&label, 0))).fill(color).finish(app.ctx()).width(10.).height(10.),
            text(id!(str_hash(&label, 1)), label).fill(FG).font_size(13).build(app.ctx()),
        ],
    )
}

fn str_hash(s: &str, salt: u64) -> u64 {
    let mut h = 0xcbf29ce484222325u64 ^ salt;
    for b in s.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

fn panel<'a>(
    panel_rect: Layout<'a, View<State>, AppCtx>,
    inner: Layout<'a, View<State>, AppCtx>,
) -> Layout<'a, View<State>, AppCtx> {
    stack(vec![panel_rect, inner.pad(12.)])
}

fn panel_bg<'a>(app: &mut AppState) -> Layout<'a, View<State>, AppCtx> {
    rect(id!())
        .fill(PANEL)
        .stroke(BORDER, Stroke::new(1.))
        .corner_rounding(8.)
        .build(app.ctx())
}

fn mappings_panel<'a>(s: &'a State, app: &mut AppState) -> Layout<'a, View<State>, AppCtx> {
    let bg = panel_bg(app);
    let title = section_title(app, "Mappings");
    let addr_row = server_addr_row(s, app);
    let buttons = buttons_section(s, app);
    let left = stick_section(s, app, StickId::Left, "Left Stick");
    let right = stick_section(s, app, StickId::Right, "Right Stick");
    let sticks_row = row_spaced(16., vec![left, right]);
    let trigs = triggers_section(s, app);
    let dp = dpad_section(s, app);
    let trigs_dpad_row = row_spaced(16., vec![trigs, dp]);
    panel(
        bg,
        column_spaced(
            14.,
            vec![
                title,
                addr_row.height(30.),
                buttons,
                sticks_row,
                trigs_dpad_row,
            ],
        ),
    )
}

fn section_title(app: &mut AppState, t: &'static str) -> Layout<'static, View<State>, AppCtx> {
    text(id!(str_hash(t, 9)), t)
        .fill(FG_DIM)
        .font_weight(FontWeight::BOLD)
        .font_size(13)
        .align(Alignment::Start)
        .build(app.ctx())
        .height(16.)
}

fn server_addr_row<'a>(s: &'a State, app: &mut AppState) -> Layout<'a, View<State>, AppCtx> {
    row_spaced(
        8.,
        vec![
            labeled(app, "Listen Addr", 90.),
            text_field_addr(s, app).expand_x(),
            labeled(app, "Port", 40.),
            text_field_port(s, app).width(70.),
        ],
    )
}

fn labeled(
    app: &mut AppState,
    label: &'static str,
    w: f32,
) -> Layout<'static, View<State>, AppCtx> {
    text(id!(str_hash(label, 13)), label)
        .fill(FG_DIM)
        .font_size(LABEL_SIZE)
        .align(Alignment::Start)
        .build(app.ctx())
        .width(w)
}

fn text_field_addr<'a>(s: &'a State, app: &mut AppState) -> Layout<'a, View<State>, AppCtx> {
    text_field(
        id!(),
        (
            s.addr_field.clone(),
            Binding::new(
                |s: &State| s.addr_field.clone(),
                |s: &mut State, v: TextState| {
                    s.mappings.listen_addr = v.text.clone();
                    s.addr_field = v;
                },
            ),
        ),
    )
    .font_size(13)
    .text_fill(FG)
    .cursor_fill(FG)
    .highlight_fill(ACCENT.with_alpha(0.4))
    .enter_end_editing()
    .esc_end_editing()
    .background(|_, _, ctx| {
        rect(id!())
            .fill(PANEL_HI)
            .stroke(BORDER, Stroke::new(1.))
            .corner_rounding(5.)
            .build(ctx)
    })
    .padding(6.)
    .on_edit(|s, app, i| {
        if let EditInteraction::End = i {
            let _ = s
                .rebind_tx
                .send((s.mappings.listen_addr.clone(), s.mappings.listen_port));
            if s.daemon.is_some() {
                stop_daemon(s);
                start_daemon(s, app);
            }
            s.persist(app);
        }
    })
    .build(app.ctx())
}

fn text_field_port<'a>(s: &'a State, app: &mut AppState) -> Layout<'a, View<State>, AppCtx> {
    text_field(
        id!(),
        (
            s.port_field.clone(),
            Binding::new(
                |s: &State| s.port_field.clone(),
                |s: &mut State, v: TextState| {
                    if let Ok(p) = v.text.trim().parse::<u16>() {
                        s.mappings.listen_port = p;
                    }
                    s.port_field = v;
                },
            ),
        ),
    )
    .font_size(13)
    .text_fill(FG)
    .cursor_fill(FG)
    .highlight_fill(ACCENT.with_alpha(0.4))
    .enter_end_editing()
    .esc_end_editing()
    .background(|_, _, ctx| {
        rect(id!())
            .fill(PANEL_HI)
            .stroke(BORDER, Stroke::new(1.))
            .corner_rounding(5.)
            .build(ctx)
    })
    .padding(6.)
    .on_edit(|s, app, i| {
        if let EditInteraction::End = i {
            s.port_field = TextState::new(s.mappings.listen_port.to_string());
            let _ = s
                .rebind_tx
                .send((s.mappings.listen_addr.clone(), s.mappings.listen_port));
            if s.daemon.is_some() {
                stop_daemon(s);
                start_daemon(s, app);
            }
            s.persist(app);
        }
    })
    .build(app.ctx())
}

fn buttons_section<'a>(s: &'a State, app: &mut AppState) -> Layout<'a, View<State>, AppCtx> {
    let rows: Vec<Layout<'a, View<State>, AppCtx>> = BUTTON_CONTROLS
        .chunks(2)
        .map(|chunk| {
            let mut items: Vec<_> = chunk.iter().map(|c| button_row(s, app, *c)).collect();
            while items.len() < 2 {
                items.push(space().width(LABEL_W + 6. + KEY_W));
            }
            row_spaced(12., items).height(ROW_H)
        })
        .collect();
    column_spaced(4., rows)
}

fn button_row<'a>(
    s: &'a State,
    app: &mut AppState,
    control: GamepadControl,
) -> Layout<'a, View<State>, AppCtx> {
    let label = text(id!(control_hash(control)), format!("{control}"))
        .fill(FG_DIM)
        .font_size(LABEL_SIZE)
        .align(Alignment::Start)
        .build(app.ctx())
        .width(LABEL_W);
    let kb = key_button(s, app, ListenSlot::Button(control)).width(KEY_W);
    row_spaced(6., vec![label, kb]).height(ROW_H)
}

fn control_hash(c: GamepadControl) -> u64 {
    str_hash(&format!("{c:?}"), 17)
}

fn key_button<'a>(
    s: &'a State,
    app: &mut AppState,
    slot: ListenSlot,
) -> Layout<'a, View<State>, AppCtx> {
    let listening = s.listening == Some(slot);
    let label: String = if listening {
        "Press a key...".into()
    } else {
        s.slot_current_key(slot)
            .map(|k| k.label().to_string())
            .unwrap_or_else(|| "—".into())
    };
    let btn_state = s.key_buttons.get(&slot).copied().unwrap_or_default();
    let (fill, border, text_col) = if listening {
        (ACCENT, ACCENT, Color::WHITE)
    } else {
        (PANEL_HI, BORDER, FG)
    };
    button(
        id!(slot_hash(slot)),
        (
            btn_state,
            Binding::new(
                move |s: &State| s.key_buttons.get(&slot).copied().unwrap_or_default(),
                move |s: &mut State, v| {
                    s.key_buttons.insert(slot, v);
                },
            ),
        ),
    )
    .surface(move |b, ctx| {
        let bg = match (b.depressed, b.hovered) {
            (true, _) => fill.map_lightness(|l| l - 0.1),
            (false, true) => fill.map_lightness(|l| l + 0.05),
            (false, false) => fill,
        };
        rect(id!())
            .fill(bg)
            .stroke(border, Stroke::new(1.))
            .corner_rounding(5.)
            .build(ctx)
    })
    .label(move |_b, ctx| {
        text(id!(), label.clone())
            .fill(text_col)
            .font_size(12)
            .build(ctx)
    })
    .on_click(move |s, _app| {
        s.listening = if s.listening == Some(slot) {
            None
        } else {
            Some(slot)
        };
    })
    .build(app.ctx())
    .expand_x()
}

fn slot_hash(slot: ListenSlot) -> u64 {
    str_hash(&format!("{slot:?}"), 23)
}

fn deadzone_control<'a>(
    _s: &'a State,
    app: &mut AppState,
    id_seed: u64,
    slider_state: SliderState,
    scale: f32,
    binding: Binding<State, SliderState>,
    persist_if_not_dragging: impl Fn(&mut State, &mut AppState) + 'static,
) -> Layout<'a, View<State>, AppCtx> {
    let value = (slider_state.value * scale) as u32;
    let label = text(id!(id_seed ^ 1), "Dead Zone")
        .fill(FG_DIM)
        .font_size(LABEL_SIZE)
        .align(Alignment::Start)
        .build(app.ctx())
        .width(LABEL_W);
    let sl = slider(id!(id_seed ^ 2), (slider_state, binding))
        .on_change(move |s, app, _| persist_if_not_dragging(s, app))
        .build(app.ctx())
        .height(18.)
        .width(SLIDER_W);
    let value_text = text(id!(id_seed ^ 3), format!("{value}"))
        .fill(FG)
        .font_size(LABEL_SIZE)
        .build(app.ctx())
        .width(VALUE_W);
    let control_group = row_spaced(5., vec![sl, value_text]).width(KEY_W);
    row_spaced(6., vec![label, control_group]).height(ROW_H)
}

fn stick_section<'a>(
    s: &'a State,
    app: &mut AppState,
    stick: StickId,
    title: &'static str,
) -> Layout<'a, View<State>, AppCtx> {
    let title_text = text(id!(str_hash(title, 21)), title)
        .fill(FG_DIM)
        .font_weight(FontWeight::BOLD)
        .font_size(13)
        .align(Alignment::Start)
        .build(app.ctx())
        .height(18.);
    let dz = deadzone_control(
        s,
        app,
        str_hash(title, 22),
        s.stick_deadzone.get(&stick).copied().unwrap_or_default(),
        128.0,
        Binding::new(
            move |s: &State| s.stick_deadzone.get(&stick).copied().unwrap_or_default(),
            move |s: &mut State, v: SliderState| {
                s.stick_deadzone.insert(stick, v);
                let dz = (v.value * 128.0) as u8;
                s.mappings
                    .sticks
                    .entry(stick)
                    .and_modify(|m| m.deadzone = dz)
                    .or_insert_with(|| {
                        let mut d = default_stick();
                        d.deadzone = dz;
                        d
                    });
            },
        ),
        move |s, app| {
            let dragging = s
                .stick_deadzone
                .get(&stick)
                .map(|sl| sl.dragging)
                .unwrap_or(false);
            if !dragging {
                s.persist(app);
            }
        },
    );
    column_spaced(
        6.,
        vec![
            title_text,
            dz,
            stick_dir(s, app, stick, StickDir::Up, "Up"),
            stick_dir(s, app, stick, StickDir::Down, "Down"),
            stick_dir(s, app, stick, StickDir::Left, "Left"),
            stick_dir(s, app, stick, StickDir::Right, "Right"),
        ],
    )
}

fn stick_dir<'a>(
    s: &'a State,
    app: &mut AppState,
    stick: StickId,
    dir: StickDir,
    label: &'static str,
) -> Layout<'a, View<State>, AppCtx> {
    let text_layout = text(id!(str_hash(label, stick_hash(stick))), label)
        .fill(FG_DIM)
        .font_size(LABEL_SIZE)
        .align(Alignment::Start)
        .build(app.ctx())
        .width(LABEL_W);
    let kb = key_button(s, app, ListenSlot::Stick(stick, dir)).width(KEY_W);
    row_spaced(6., vec![text_layout, kb]).height(ROW_H)
}

fn stick_hash(stick: StickId) -> u64 {
    match stick {
        StickId::Left => 41,
        StickId::Right => 43,
    }
}

fn triggers_section<'a>(s: &'a State, app: &mut AppState) -> Layout<'a, View<State>, AppCtx> {
    let title = text(id!(str_hash("Triggers", 21)), "Triggers")
        .fill(FG_DIM)
        .font_weight(FontWeight::BOLD)
        .font_size(13)
        .align(Alignment::Start)
        .build(app.ctx())
        .height(18.);
    column_spaced(
        6.,
        vec![
            title,
            trigger_key_row(s, app, GamepadControl::L2, "L2"),
            trigger_dz_row(s, app, GamepadControl::L2),
            trigger_key_row(s, app, GamepadControl::R2, "R2"),
            trigger_dz_row(s, app, GamepadControl::R2),
        ],
    )
}

fn trigger_key_row<'a>(
    s: &'a State,
    app: &mut AppState,
    control: GamepadControl,
    label: &'static str,
) -> Layout<'a, View<State>, AppCtx> {
    let label_text = text(id!(str_hash(label, 61)), label)
        .fill(FG_DIM)
        .font_size(LABEL_SIZE)
        .align(Alignment::Start)
        .build(app.ctx())
        .width(LABEL_W);
    let kb = key_button(s, app, ListenSlot::Trigger(control)).width(KEY_W);
    row_spaced(6., vec![label_text, kb]).height(ROW_H)
}

fn trigger_dz_row<'a>(
    s: &'a State,
    app: &mut AppState,
    control: GamepadControl,
) -> Layout<'a, View<State>, AppCtx> {
    let slider_state = s
        .trigger_deadzone
        .get(&control)
        .copied()
        .unwrap_or_default();
    deadzone_control(
        s,
        app,
        str_hash(&format!("{control:?}"), 62),
        slider_state,
        255.0,
        Binding::new(
            move |s: &State| {
                s.trigger_deadzone
                    .get(&control)
                    .copied()
                    .unwrap_or_default()
            },
            move |s: &mut State, v: SliderState| {
                s.trigger_deadzone.insert(control, v);
                let dz = (v.value * 255.0) as u8;
                s.mappings
                    .triggers
                    .entry(control)
                    .and_modify(|t| t.deadzone = dz)
                    .or_insert(TriggerMapping {
                        key: KeyCode::C,
                        deadzone: dz,
                    });
            },
        ),
        move |s, app| {
            let dragging = s
                .trigger_deadzone
                .get(&control)
                .map(|sl| sl.dragging)
                .unwrap_or(false);
            if !dragging {
                s.persist(app);
            }
        },
    )
}

fn dpad_section<'a>(s: &'a State, app: &mut AppState) -> Layout<'a, View<State>, AppCtx> {
    column_spaced(
        4.,
        vec![
            section_title(app, "D-Pad"),
            dpad_dir(s, app, DPadDir::Up, "Up"),
            dpad_dir(s, app, DPadDir::Down, "Down"),
            dpad_dir(s, app, DPadDir::Left, "Left"),
            dpad_dir(s, app, DPadDir::Right, "Right"),
        ],
    )
}

fn dpad_dir<'a>(
    s: &'a State,
    app: &mut AppState,
    dir: DPadDir,
    label: &'static str,
) -> Layout<'a, View<State>, AppCtx> {
    let label_text = text(id!(str_hash(label, 71)), label)
        .fill(FG_DIM)
        .font_size(LABEL_SIZE)
        .align(Alignment::Start)
        .build(app.ctx())
        .width(LABEL_W);
    let kb = key_button(s, app, ListenSlot::DPad(dir)).width(KEY_W);
    row_spaced(6., vec![label_text, kb]).height(ROW_H)
}

fn setup_panel<'a>(s: &'a State, app: &mut AppState) -> Layout<'a, View<State>, AppCtx> {
    let bg = panel_bg(app);
    let title = section_title(app, "USB Setup");
    let dd_row = device_dropdown(s, app);
    let btn_row = setup_buttons(s, app);
    let help = text(
        id!(),
        "Pick a gamepad bus ID, bind it (elevated), then attach it to WSL.",
    )
    .fill(FG_DIM)
    .font_size(11)
    .wrap()
    .build(app.ctx());
    panel(
        bg,
        column_spaced(
            10.,
            vec![title, dd_row.height(30.), btn_row.height(30.), help],
        ),
    )
}

fn device_dropdown<'a>(s: &'a State, app: &mut AppState) -> Layout<'a, View<State>, AppCtx> {
    let devs = s.devices.clone();
    let options: Vec<String> = if devs.is_empty() {
        vec![String::new()]
    } else {
        devs.iter().map(|d| d.busid.clone()).collect()
    };
    let state_cloned = s.device_dd.clone();
    dropdown(
        id!(),
        (
            state_cloned,
            Binding::new(
                |s: &State| s.device_dd.clone(),
                |s: &mut State, v: DropdownState<String>| {
                    s.mappings.last_busid = Some(v.selected.clone()).filter(|b| !b.is_empty());
                    s.device_dd = v;
                },
            ),
        ),
        options,
        move |item, ctx| {
            let dev = devs.iter().find(|d| d.busid == *item.value);
            let label = match dev {
                Some(d) => format!("{}  {}  [{}]", d.busid, d.description, d.state),
                None if item.value.is_empty() => "no devices found".to_string(),
                None => item.value.clone(),
            };
            text(id!(item.index as u64), label)
                .fill(if item.selected { ACCENT } else { FG })
                .font_size(12)
                .build(ctx)
                .pad_x(8.)
                .pad_y(4.)
        },
    )
    .background(|ds, ctx| {
        rect(id!())
            .fill(if ds.expanded { PANEL_HI } else { PANEL })
            .stroke(if ds.expanded { ACCENT } else { BORDER }, Stroke::new(1.))
            .corner_rounding(5.)
            .build(ctx)
    })
    .on_select(|s, app, _| s.persist(app))
    .build(app.ctx())
}

fn setup_buttons<'a>(s: &'a State, app: &mut AppState) -> Layout<'a, View<State>, AppCtx> {
    row_spaced(
        6.,
        vec![
            action_button(
                s.refresh_btn,
                app,
                "Refresh",
                Binding::new(|s: &State| s.refresh_btn, |s: &mut State, v| s.refresh_btn = v),
                |_s, app| {
                    let cb = app.callback(|s: &mut State, devs: Vec<setup::UsbDevice>| {
                        s.devices = devs;
                        if s.device_dd.selected.is_empty()
                            && let Some(f) = s.devices.first()
                        {
                            s.device_dd.selected = f.busid.clone();
                        }
                    });
                    app.spawn(async move {
                        let devs = tokio::task::spawn_blocking(setup::list_devices)
                            .await
                            .ok()
                            .and_then(|r| r.ok())
                            .unwrap_or_default();
                        cb.send(devs);
                    });
                },
            ),
            action_button(
                s.bind_btn,
                app,
                "Bind",
                Binding::new(|s: &State| s.bind_btn, |s: &mut State, v| s.bind_btn = v),
                |s, app| {
                    let bus = s.device_dd.selected.clone();
                    let cb = app.callback(|s: &mut State, r: Result<String, String>| match r {
                        Ok(bus) => s.push_log(LogEntry::Info(format!("bound {bus}"))),
                        Err(e) => s.push_log(LogEntry::Error(format!("bind: {e}"))),
                    });
                    app.spawn(async move {
                        let bus2 = bus.clone();
                        let r = tokio::task::spawn_blocking(move || setup::bind(&bus2))
                            .await
                            .ok()
                            .unwrap_or_else(|| Err("task panic".into()))
                            .map(|_| bus);
                        cb.send(r);
                    });
                },
            ),
            action_button(
                s.attach_btn,
                app,
                "Attach WSL",
                Binding::new(|s: &State| s.attach_btn, |s: &mut State, v| s.attach_btn = v),
                |s, app| {
                    let bus = s.device_dd.selected.clone();
                    let cb = app.callback(|s: &mut State, r: Result<String, String>| match r {
                        Ok(bus) => s.push_log(LogEntry::Info(format!("attached {bus} to WSL"))),
                        Err(e) => s.push_log(LogEntry::Error(format!("attach: {e}"))),
                    });
                    app.spawn(async move {
                        let bus2 = bus.clone();
                        let r = tokio::task::spawn_blocking(move || setup::attach(&bus2))
                            .await
                            .ok()
                            .unwrap_or_else(|| Err("task panic".into()))
                            .map(|_| bus);
                        cb.send(r);
                    });
                },
            ),
            action_button(
                s.detach_btn,
                app,
                "Detach",
                Binding::new(|s: &State| s.detach_btn, |s: &mut State, v| s.detach_btn = v),
                |s, app| {
                    let bus = s.device_dd.selected.clone();
                    let cb = app.callback(|s: &mut State, r: Result<String, String>| match r {
                        Ok(bus) => s.push_log(LogEntry::Info(format!("detached {bus}"))),
                        Err(e) => s.push_log(LogEntry::Error(format!("detach: {e}"))),
                    });
                    app.spawn(async move {
                        let bus2 = bus.clone();
                        let r = tokio::task::spawn_blocking(move || setup::detach(&bus2))
                            .await
                            .ok()
                            .unwrap_or_else(|| Err("task panic".into()))
                            .map(|_| bus);
                        cb.send(r);
                    });
                },
            ),
        ],
    )
}

fn action_button<'a>(
    current: ButtonState,
    app: &mut AppState,
    label: &'static str,
    binding: Binding<State, ButtonState>,
    on_click: impl Fn(&mut State, &mut AppState) + 'static,
) -> Layout<'a, View<State>, AppCtx> {
    button(id!(str_hash(label, 91)), (current, binding))
        .surface(|btn, ctx| {
            rect(id!())
                .fill(match (btn.depressed, btn.hovered) {
                    (true, _) => ACCENT.map_lightness(|l| l - 0.15),
                    (false, true) => ACCENT.map_lightness(|l| l + 0.05),
                    (false, false) => ACCENT,
                })
                .corner_rounding(5.)
                .build(ctx)
        })
        .text_label(label)
        .on_click(on_click)
        .build(app.ctx())
        .expand_x()
}

fn log_panel<'a>(s: &'a State, app: &mut AppState) -> Layout<'a, View<State>, AppCtx> {
    let bg = panel_bg(app);
    let title = section_title(app, "Activity");
    let entries: Vec<LogEntry> = s.log.iter().rev().cloned().collect();
    let scroller_state = s.log_scroller.clone();
    let view = scroller(
        id!(),
        None,
        scroller_state,
        move |index, _id, ctx| {
            entries.get(index).map(|entry| {
                let (color, msg) = match entry {
                    LogEntry::Info(m) => (FG_DIM, m.as_str()),
                    LogEntry::Key(m) => (KEY_COLOR, m.as_str()),
                    LogEntry::Warn(m) => (YELLOW, m.as_str()),
                    LogEntry::Error(m) => (RED, m.as_str()),
                };
                text(id!(index as u64), msg)
                    .fill(color)
                    .font_size(11)
                    .build(ctx)
                    .height(14.)
                    .pad_x(4.)
            })
        },
        app.ctx(),
    );
    panel(bg, column_spaced(6., vec![title, view.expand()]))
}

const AXES: &[GamepadControl] = &[
    GamepadControl::LeftStickX,
    GamepadControl::LeftStickY,
    GamepadControl::RightStickX,
    GamepadControl::RightStickY,
    GamepadControl::L2,
    GamepadControl::R2,
    GamepadControl::DPadX,
    GamepadControl::DPadY,
];

fn gamepad_panel<'a>(s: &'a State, app: &mut AppState) -> Layout<'a, View<State>, AppCtx> {
    let bg = panel_bg(app);
    let title = section_title(app, "Gamepad Client");
    let gp_dd = gp_device_dropdown(s, app).expand_x();
    let gp_refresh = action_button(
        s.gp_refresh_btn,
        app,
        "Refresh",
        Binding::new(
            |s: &State| s.gp_refresh_btn,
            |s, v| s.gp_refresh_btn = v,
        ),
        |_s, app| refresh_gp_devices(app),
    )
    .width(80.);
    let running = s.daemon.is_some();
    let start = action_button(
        s.start_btn,
        app,
        if running { "Restart" } else { "Start" },
        Binding::new(|s: &State| s.start_btn, |s, v| s.start_btn = v),
        |s, app| {
            if s.daemon.is_some() {
                stop_daemon(s);
            }
            s.as_root = false;
            start_daemon(s, app);
        },
    )
    .width(80.);
    let stop = action_button(
        s.stop_btn,
        app,
        "Stop",
        Binding::new(|s: &State| s.stop_btn, |s, v| s.stop_btn = v),
        |s, _app| stop_daemon(s),
    )
    .width(60.);
    let control_row = row_spaced(6., vec![gp_dd, gp_refresh, start, stop]).height(30.);

    let buttons_grid: Vec<Layout<'a, View<State>, AppCtx>> = BUTTON_CONTROLS
        .chunks(5)
        .map(|chunk| {
            let cells: Vec<_> = chunk.iter().map(|c| button_cell(s, app, *c)).collect();
            row_spaced(4., cells).height(22.)
        })
        .collect();

    let axes_rows: Vec<Layout<'a, View<State>, AppCtx>> =
        AXES.iter().map(|c| axis_row(s, app, *c)).collect();

    panel(
        bg,
        column_spaced(
            8.,
            vec![
                title,
                control_row,
                column_spaced(4., buttons_grid),
                column_spaced(3., axes_rows).expand(),
            ],
        ),
    )
}

fn gp_device_dropdown<'a>(s: &'a State, app: &mut AppState) -> Layout<'a, View<State>, AppCtx> {
    let devs: Vec<(String, String)> = s.gp_devices.clone();
    let options: Vec<String> = if devs.is_empty() {
        vec![String::new()]
    } else {
        devs.iter().map(|(p, _)| p.clone()).collect()
    };
    dropdown(
        id!(),
        (
            s.gp_device_dd.clone(),
            Binding::new(
                |s: &State| s.gp_device_dd.clone(),
                |s: &mut State, v: DropdownState<String>| {
                    s.mappings.last_device = Some(v.selected.clone()).filter(|p| !p.is_empty());
                    s.gp_device_dd = v;
                },
            ),
        ),
        options,
        move |item, ctx| {
            let label = if item.value.is_empty() {
                "no gamepad found in WSL".to_string()
            } else {
                let name = devs
                    .iter()
                    .find(|(p, _)| p == item.value)
                    .map(|(_, n)| n.as_str())
                    .unwrap_or("");
                format!("{}   {}", item.value, name)
            };
            text(id!(item.index as u64), label)
                .fill(if item.selected { ACCENT } else { FG })
                .font_size(12)
                .build(ctx)
                .pad_x(8.)
                .pad_y(4.)
        },
    )
    .background(|ds, ctx| {
        rect(id!())
            .fill(if ds.expanded { PANEL_HI } else { PANEL })
            .stroke(if ds.expanded { ACCENT } else { BORDER }, Stroke::new(1.))
            .corner_rounding(5.)
            .build(ctx)
    })
    .on_select(|s, app, _| s.persist(app))
    .build(app.ctx())
}

fn button_cell<'a>(
    s: &'a State,
    app: &mut AppState,
    c: GamepadControl,
) -> Layout<'a, View<State>, AppCtx> {
    let pressed = s.live_buttons.get(&c).copied().unwrap_or(false);
    let (fill, stroke_col) = if pressed {
        (ACCENT, ACCENT)
    } else {
        (PANEL_HI, BORDER)
    };
    let bg = rect(id!(control_hash(c)))
        .fill(fill)
        .stroke(stroke_col, Stroke::new(1.))
        .corner_rounding(4.)
        .build(app.ctx());
    let label = text(id!(control_hash(c) ^ 0xAA), format!("{c}"))
        .fill(if pressed { Color::WHITE } else { FG })
        .font_size(10)
        .build(app.ctx());
    stack(vec![bg, label]).expand_x()
}

fn axis_row<'a>(
    s: &'a State,
    app: &mut AppState,
    c: GamepadControl,
) -> Layout<'a, View<State>, AppCtx> {
    let value = s.live_axes.get(&c).copied().unwrap_or(0);
    let label = text(id!(control_hash(c) ^ 0xA0), format!("{c}"))
        .fill(FG_DIM)
        .font_size(10)
        .build(app.ctx())
        .width(90.);
    let val_label = text(id!(control_hash(c) ^ 0xA1), format!("{value:>5}"))
        .fill(FG)
        .font_size(10)
        .build(app.ctx())
        .width(40.);
    let bar_bg = rect(id!(control_hash(c) ^ 0xA2))
        .fill(PANEL_HI)
        .corner_rounding(3.)
        .build(app.ctx());
    let (is_signed, norm) = if matches!(c, GamepadControl::L2 | GamepadControl::R2) {
        (false, (value as f32 / 255.0).clamp(0.0, 1.0))
    } else if matches!(c, GamepadControl::DPadX | GamepadControl::DPadY) {
        let v = value as i8 as f32;
        (true, v.clamp(-1.0, 1.0))
    } else {
        let v = (value as f32 - 127.0) / 127.0;
        (true, v.clamp(-1.0, 1.0))
    };
    let bar = path(id!(control_hash(c) ^ 0xA3), move |area: Area| {
        let h = area.height.min(10.0) as f64;
        let y0 = (area.y + (area.height - h as f32) / 2.0) as f64;
        let (x0, w) = if is_signed {
            let center = area.x + area.width / 2.0;
            if norm >= 0.0 {
                (center as f64, (area.width / 2.0 * norm) as f64)
            } else {
                let w = (area.width / 2.0 * -norm) as f64;
                ((center as f64) - w, w)
            }
        } else {
            (area.x as f64, (area.width * norm) as f64)
        };
        let w = w.max(0.0);
        let mut p = BezPath::new();
        p.move_to((x0, y0));
        p.line_to((x0 + w, y0));
        p.line_to((x0 + w, y0 + h));
        p.line_to((x0, y0 + h));
        p.close_path();
        p
    })
    .fill(ACCENT)
    .build(app.ctx());
    let bar_stack = stack(vec![bar_bg, bar]).height(12.).expand_x();
    row_spaced(6., vec![label, val_label, bar_stack]).height(14.)
}
