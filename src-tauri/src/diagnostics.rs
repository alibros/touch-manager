use parking_lot::Mutex;
use serde::Serialize;
use std::{
    collections::HashMap,
    io::{Read, Write},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::Duration,
};
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

pub struct ConsoleManager {
    sessions: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl ConsoleManager {
    pub fn new() -> Self {
        Self {
            sessions: Mutex::new(HashMap::new()),
        }
    }

    pub fn start(
        &self,
        app: AppHandle,
        port_name: String,
        baud_rate: u32,
    ) -> Result<String, String> {
        let mut port = serialport::new(&port_name, baud_rate)
            .timeout(Duration::from_millis(100))
            .open()
            .map_err(|error| error.to_string())?;
        let session_id = Uuid::new_v4().to_string();
        let running = Arc::new(AtomicBool::new(true));
        self.sessions
            .lock()
            .insert(session_id.clone(), running.clone());
        let event_session = session_id.clone();

        thread::spawn(move || {
            let mut buffer = [0_u8; 1024];
            let mut pending = Vec::new();
            while running.load(Ordering::Relaxed) {
                match port.read(&mut buffer) {
                    Ok(count) if count > 0 => {
                        pending.extend_from_slice(&buffer[..count]);
                        while let Some(index) = pending.iter().position(|byte| *byte == b'\n') {
                            let line = pending.drain(..=index).collect::<Vec<_>>();
                            let _ = app.emit(
                                "serial-line",
                                SerialLine {
                                    session_id: event_session.clone(),
                                    line: String::from_utf8_lossy(&line).trim_end().to_string(),
                                },
                            );
                        }
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::TimedOut => {}
                    Err(error) => {
                        let _ = app.emit(
                            "serial-line",
                            SerialLine {
                                session_id: event_session.clone(),
                                line: format!("[serial error] {error}"),
                            },
                        );
                        break;
                    }
                }
            }
        });

        Ok(session_id)
    }

    pub fn stop(&self, session_id: &str) -> bool {
        self.sessions
            .lock()
            .remove(session_id)
            .is_some_and(|running| {
                running.store(false, Ordering::Relaxed);
                true
            })
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SerialLine {
    session_id: String,
    line: String,
}

pub fn request_update_mode(port_name: &str) -> Result<String, String> {
    let mut port = serialport::new(port_name, 115_200)
        .timeout(Duration::from_millis(500))
        .open()
        .map_err(|error| error.to_string())?;
    let nonce = Uuid::new_v4().simple().to_string();
    let command = format!("TM1 ENTER_DFU {nonce}\n");
    port.write_all(command.as_bytes())
        .map_err(|error| error.to_string())?;
    port.flush().map_err(|error| error.to_string())?;
    Ok(nonce)
}
