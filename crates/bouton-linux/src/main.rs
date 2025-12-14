mod gamepad;
mod socket_client;
mod ui;


use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use gamepad::GamepadReader;
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use socket_client::SocketClient;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use ui::GamepadState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <gamepad_device> [server_addr]", args[0]);
        eprintln!("Example: {} /dev/input/event0 127.0.0.1:8000", args[0]);
        std::process::exit(1);
    }

    let gamepad_path = &args[1];
    let server_addr: SocketAddr = if args.len() > 2 {
        args[2].parse()?
    } else {
        "127.0.0.1:8000".parse()?
    };

    let mut gamepad = match GamepadReader::open(gamepad_path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("Error opening gamepad at {}: {}", gamepad_path, e);
            eprintln!();
            eprintln!("Make sure the gamepad device exists. You can find it with:");
            eprintln!("  ls /dev/input/event*");
            std::process::exit(1);
        }
    };

    // Setup terminal first
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut state = GamepadState::new();

    // Spawn background task to connect to server
    let client = Arc::new(Mutex::new(None));
    let client_clone = Arc::clone(&client);
    let state_server_status = Arc::new(Mutex::new(ui::ConnectionState::Connecting));
    let state_server_status_clone = Arc::clone(&state_server_status);
    
    tokio::spawn(async move {
        match SocketClient::connect(server_addr).await {
            Ok(socket_client) => {
                *client_clone.lock().await = Some(socket_client);
                *state_server_status_clone.lock().await = ui::ConnectionState::Connected;
            }
            Err(_) => {
                *state_server_status_clone.lock().await = ui::ConnectionState::Error;
            }
        }
    });

    let mut last_server_retry = Instant::now();

    // Main loop
    loop {
        // Handle input events
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                    break;
                }
            }
        }

        // Update server connection status from background task
        state.server_state = *state_server_status.lock().await;

        // Retry server connection if it failed and 1 second has passed
        if state.server_state == ui::ConnectionState::Error && last_server_retry.elapsed() >= Duration::from_secs(1) {
            last_server_retry = Instant::now();
            *state_server_status.lock().await = ui::ConnectionState::Connecting;
            
            let client_clone = Arc::clone(&client);
            let state_server_status_clone = Arc::clone(&state_server_status);
            
            tokio::spawn(async move {
                match SocketClient::connect(server_addr).await {
                    Ok(socket_client) => {
                        *client_clone.lock().await = Some(socket_client);
                        *state_server_status_clone.lock().await = ui::ConnectionState::Connected;
                    }
                    Err(_) => {
                        *state_server_status_clone.lock().await = ui::ConnectionState::Error;
                    }
                }
            });
        }

        // Read gamepad events
        match gamepad.fetch_events() {
            Ok(events) => {
                state.gamepad_state = ui::ConnectionState::Connected;
                state.gamepad_error = None;
                
                for event in events {
                    state.update(&event);

                    let mut client_guard = client.lock().await;
                    if let Some(ref mut c) = client_guard.as_mut() {
                        if c.send_event(event).await.is_err() {
                            // Server disconnected
                            *client_guard = None;
                            *state_server_status.lock().await = ui::ConnectionState::Error;
                        }
                    }
                }
            }
            Err(e) => {
                state.gamepad_state = ui::ConnectionState::Error;
                state.gamepad_error = Some(e.to_string());
            }
        }

        // Render UI (always, even if no events)
        terminal.draw(|f| {
            ui::draw(f, &state);
        })?;
    }

    // Cleanup terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    Ok(())
}
