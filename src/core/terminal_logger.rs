use chrono::Local;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

/// Dev-only terminal logger.
/// Only compiled in debug builds. Release builds get no-ops.

/// Determine the log file path based on platform.
#[cfg(debug_assertions)]
fn get_log_path() -> PathBuf {
    let base = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("igris");
    let _ = fs::create_dir_all(&base);
    base.join("igris_terminal.log")
}

/// Truncate a string to at most `max_chars` characters on a valid UTF-8 boundary.
/// Returns the slice and the total character count.
#[cfg(debug_assertions)]
fn truncate_on_char_boundary(s: &str, max_chars: usize) -> (&str, usize) {
    let total = s.chars().count();
    match s.char_indices().nth(max_chars) {
        Some((idx, _)) => (&s[..idx], total),
        None => (s, total),
    }
}

/// Write a single line to the log file.
#[cfg(debug_assertions)]
fn write_log(level: &str, message: &str) {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    let entry = format!("[{}] [{}] {}\n", timestamp, level, message);
    let path = get_log_path();
    if let Ok(mut file) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path.as_path())
    {
        let _ = file.write_all(entry.as_bytes());
    }
}

/// Log user input (typed or spoken).
#[cfg(debug_assertions)]
pub fn log_input(input: &str) {
    write_log("USER", input);
}

/// Log assistant text output (from message field).
#[cfg(debug_assertions)]
pub fn log_output(output: &str) {
    write_log("IGRIS", output);
}

/// Log a shell command being executed.
#[cfg(debug_assertions)]
pub fn log_shell_command(command: &str) {
    write_log("SHELL_CMD", command);
}

/// Log a shell command result (output or error, first N chars).
#[cfg(debug_assertions)]
pub fn log_shell_result(result: &str, _truncated: bool) {
    let max_chars = 2000;
    let (head, total) = truncate_on_char_boundary(result, max_chars);
    let display = if total > max_chars {
        format!("{}... [truncated, {} total chars]", head, total)
    } else {
        result.to_string()
    };
    write_log("SHELL_RES", &display);
}

/// Log a generic event.
#[cfg(debug_assertions)]
pub fn log_event(event: &str) {
    write_log("EVENT", event);
}

/// Log startup banner.
#[cfg(debug_assertions)]
pub fn log_session_start(version: &str) {
    write_log("SESSION", &format!("IGRIS v{} started", version));
}

/// Log shutdown.
#[cfg(debug_assertions)]
pub fn log_session_end(version: &str) {
    write_log("SESSION", &format!("IGRIS v{} stopped", version));
}

// ========== No-op stubs for release builds ==========

#[cfg(not(debug_assertions))]
pub fn log_input(_input: &str) {}

#[cfg(not(debug_assertions))]
pub fn log_output(_output: &str) {}

#[cfg(not(debug_assertions))]
pub fn log_shell_command(_command: &str) {}

#[cfg(not(debug_assertions))]
pub fn log_shell_result(_result: &str, _truncated: bool) {}

#[cfg(not(debug_assertions))]
pub fn log_event(_event: &str) {}

#[cfg(not(debug_assertions))]
pub fn log_session_start(_version: &str) {}

#[cfg(not(debug_assertions))]
pub fn log_session_end(_version: &str) {}
