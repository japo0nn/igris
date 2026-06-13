use std::fs::OpenOptions;
use std::io::Write;
use chrono::Local;

const LOG_FILE: &str = "supervisor.log";

pub enum SupervisorEvent {
    Startup,
    Shutdown,
    Restart { reason: String },
    Error { message: String },
}

pub fn log_event(event: SupervisorEvent) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let entry = match event {
        SupervisorEvent::Startup => {
            format!("[{}] [STARTUP] IGRIS started\n", timestamp)
        }
        SupervisorEvent::Shutdown => {
            format!("[{}] [SHUTDOWN] IGRIS stopped\n", timestamp)
        }
        SupervisorEvent::Restart { reason } => {
            format!("[{}] [RESTART] IGRIS restarted. Reason: {}\n", timestamp, reason)
        }
        SupervisorEvent::Error { message } => {
            format!("[{}] [ERROR] {}\n", timestamp, message)
        }
    };

    // Write to file only — NO Memory Layer access
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open(LOG_FILE)
    {
        let _ = file.write_all(entry.as_bytes());
    }

    // Also print to stderr for visibility
    eprint!("{}", entry);
}
