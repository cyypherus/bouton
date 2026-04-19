use bouton_core::{ControlEvent, KeyAction, control::GamepadControl};
use serde::Serialize;
use std::io::{BufWriter, Write};
use std::net::SocketAddr;
use std::path::Path;
use tokio::net::UdpSocket;

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Event<'a> {
    Device { path: &'a str, name: &'a str },
    Opened,
    PermissionDenied,
    OpenError { msg: String },
    SendError { msg: String },
    Button { control: GamepadControl, pressed: bool },
    Axis { control: GamepadControl, value: i32 },
}

fn emit(ev: Event<'_>) {
    let mut out = BufWriter::new(std::io::stdout().lock());
    let _ = serde_json::to_writer(&mut out, &ev);
    let _ = out.write_all(b"\n");
    let _ = out.flush();
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(|s| s.as_str()) {
        Some("--list") => list_devices(),
        Some("--run") => {
            let device = args.get(2).cloned().unwrap_or_default();
            let server = args.get(3).cloned().unwrap_or_default();
            run(device, server);
        }
        _ => {
            eprintln!("usage: bouton-linux --list");
            eprintln!("       bouton-linux --run <device> <host:port>");
            std::process::exit(2);
        }
    }
}

fn list_devices() {
    let Ok(entries) = std::fs::read_dir("/dev/input") else {
        return;
    };
    let mut devs: Vec<_> = entries
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let fname = p.file_name()?.to_str()?;
            if !fname.starts_with("event") {
                return None;
            }
            let name = evdev::Device::open(&p)
                .ok()
                .and_then(|d| d.name().map(|s| s.to_string()))
                .unwrap_or_else(|| fname.to_string());
            Some((p.display().to_string(), name))
        })
        .collect();
    devs.sort();
    for (path, name) in &devs {
        emit(Event::Device { path, name });
    }
}

fn run(device: String, server: String) {
    let addr: SocketAddr = match server.parse() {
        Ok(a) => a,
        Err(e) => {
            emit(Event::OpenError {
                msg: format!("bad server addr {server}: {e}"),
            });
            std::process::exit(1);
        }
    };

    let mut evdev_device = match evdev::Device::open(Path::new(&device)) {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            emit(Event::PermissionDenied);
            std::process::exit(2);
        }
        Err(e) => {
            emit(Event::OpenError { msg: e.to_string() });
            std::process::exit(1);
        }
    };
    emit(Event::Opened);

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async move {
        let socket = match UdpSocket::bind("0.0.0.0:0").await {
            Ok(s) => s,
            Err(e) => {
                emit(Event::OpenError { msg: format!("bind: {e}") });
                return;
            }
        };

        loop {
            let events = match evdev_device.fetch_events() {
                Ok(e) => e,
                Err(e) => {
                    emit(Event::OpenError { msg: e.to_string() });
                    return;
                }
            };
            for ev in events {
                let Some(ge) = bouton_core::GamepadEvent::from_evdev(ev) else {
                    continue;
                };
                let Some(ce) = ge.to_control() else { continue };
                match &ce {
                    ControlEvent::Button(b) => emit(Event::Button {
                        control: b.control,
                        pressed: matches!(b.action, KeyAction::Press),
                    }),
                    ControlEvent::Axis(a) => emit(Event::Axis {
                        control: a.control,
                        value: a.value,
                    }),
                }
                if let Ok(bytes) = bincode::serialize(&ce)
                    && let Err(e) = socket.send_to(&bytes, addr).await
                {
                    emit(Event::SendError { msg: e.to_string() });
                }
            }
        }
    });
}
