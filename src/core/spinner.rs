use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::Duration;
use crate::core::markdown::render_markdown;

const SPINNER_CHARS: &[u8] = b"-\\|/";

#[derive(Debug, Clone)]
pub struct Spinner {
    running: Arc<AtomicBool>,
    message: Arc<Mutex<String>>,
    log_lines: Arc<Mutex<Vec<String>>>,
    line_count: Arc<Mutex<usize>>,
    last_full_output: Arc<Mutex<String>>,
}

impl Spinner {
    pub fn new() -> Self {
        Spinner {
            running: Arc::new(AtomicBool::new(false)),
            message: Arc::new(Mutex::new(String::new())),
            log_lines: Arc::new(Mutex::new(Vec::new())),
            line_count: Arc::new(Mutex::new(0)),
            last_full_output: Arc::new(Mutex::new(String::new())),
        }
    }

    pub async fn start(&self, initial_message: String) {
        // Reset log lines and line count from previous session
        {
            let mut ll = self.log_lines.lock().unwrap();
            ll.clear();
        }
        {
            let mut lc = self.line_count.lock().unwrap();
            *lc = 0;
        }
        self.running.store(true, Ordering::SeqCst);
        {
            let mut msg = self.message.lock().unwrap();
            *msg = initial_message;
        }
        let running = self.running.clone();
        let message = self.message.clone();
        let log_lines = self.log_lines.clone();
        let line_count = self.line_count.clone();
        tokio::spawn(async move {
            let mut idx = 0usize;
            let mut stderr = std::io::stderr();
            let mut first = true;
            while running.load(Ordering::SeqCst) {
                let ch = SPINNER_CHARS[idx % SPINNER_CHARS.len()] as char;
                let msg = { message.lock().unwrap().clone() };
                let lines = { log_lines.lock().unwrap().clone() };
                let prev_lines = { *line_count.lock().unwrap() };
                let cur_lines = lines.len();

                if !first && prev_lines > 0 {
                    write!(stderr, "\r\x1b[{}A", prev_lines).ok();
                }
                first = false;

                // Print log lines (with | prefix)
                for line in &lines {
                    writeln!(stderr, "{}", line).ok();
                }
                // Print spinner line
                write!(stderr, "\x1b[36m{}\x1b[0m \x1b[1m{}\x1b[0m\n", ch, msg).ok();
                stderr.flush().ok();

                *line_count.lock().unwrap() = cur_lines + 1;
                idx += 1;
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        });
    }

    pub fn set_message(&self, msg: String) {
        let mut m = self.message.lock().unwrap();
        *m = msg;
    }

    pub fn add_log_line(&self, line: String) {
        let mut l = self.log_lines.lock().unwrap();
        l.push(line);
    }

    pub fn set_last_full_output(&self, output: String) {
        let mut out = self.last_full_output.lock().unwrap();
        *out = output;
    }

    pub fn get_last_full_output(&self) -> String {
        self.last_full_output.lock().unwrap().clone()
    }

    /// Clear previous log lines and reset line count for a new round.
    /// Prints escape sequences to erase the previous block from terminal.
    pub fn begin_round(&self) {
        let prev = {
            let mut lc = self.line_count.lock().unwrap();
            let val = *lc;
            *lc = 0;
            val
        };
        {
            let mut ll = self.log_lines.lock().unwrap();
            ll.clear();
        }
        if prev > 0 {
            let mut stderr = std::io::stderr();
            // Move up and clear each line
            for _ in 0..prev {
                write!(stderr, "\r\x1b[K\x1b[1A").ok();
            }
            // Clear the first line as well (spinner line)
            write!(stderr, "\r\x1b[K").ok();
            stderr.flush().ok();
        }
    }

    pub async fn stop(&self, _answer: String) {
        self.running.store(false, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(50)).await;
        // Render markdown and print final answer to stdout
        let rendered = render_markdown(&_answer);
        let mut stdout = std::io::stdout();
        writeln!(stdout, "{}", rendered).ok();
        stdout.flush().ok();
    }
}
