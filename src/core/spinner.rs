use std::io::{IsTerminal, Write};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::core::markdown::render_markdown;

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

// ── публичный интерфейс ──────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Spinner {
    running: Arc<AtomicBool>,
    message: Arc<Mutex<String>>,
    last_full_output: Arc<Mutex<String>>,
    is_tty: bool,
}

impl Spinner {
    pub fn new() -> Self {
        Spinner {
            running: Arc::new(AtomicBool::new(false)),
            message: Arc::new(Mutex::new(String::new())),
            last_full_output: Arc::new(Mutex::new(String::new())),
            is_tty: std::io::stderr().is_terminal(),
        }
    }

    /// Запустить спиннер с начальным сообщением.
    pub async fn start(&self, initial_message: String) {
        self.running.store(true, Ordering::SeqCst);
        {
            let mut msg = self.message.lock().unwrap_or_else(|e| e.into_inner());
            *msg = initial_message.clone();
        }

        let running = self.running.clone();
        let message = self.message.clone();
        let is_tty = self.is_tty;

        tokio::spawn(async move {
            if !is_tty {
                // Не TTY: напечатать один раз и ждать
                eprintln!("\x1b[2m… {}\x1b[0m", {
                    message.lock().unwrap_or_else(|e| e.into_inner()).clone()
                });
                while running.load(Ordering::SeqCst) {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
                return;
            }

            // TTY: крутить спиннер на одной строке через \r
            let mut idx = 0usize;
            while running.load(Ordering::SeqCst) {
                let frame = SPINNER_FRAMES[idx % SPINNER_FRAMES.len()];
                let msg = { message.lock().unwrap_or_else(|e| e.into_inner()).clone() };
                // \r — вернуться в начало строки, \x1b[K — стереть до конца
                eprint!("\r\x1b[K\x1b[36m{}\x1b[0m \x1b[1m{}\x1b[0m", frame, msg);
                let _ = std::io::stderr().flush();
                idx += 1;
                tokio::time::sleep(Duration::from_millis(100)).await;
            }
            // Стереть строку спиннера перед выходом
            eprint!("\r\x1b[K");
            let _ = std::io::stderr().flush();
        });
    }

    /// Обновить сообщение спиннера на лету.
    pub fn set_message(&self, msg: String) {
        *self.message.lock().unwrap_or_else(|e| e.into_inner()) = msg;
    }

    /// Вывести строку прогресса (лог тулзы/экшена).
    /// В TTY — сначала стирает спиннер, печатает лог, затем спиннер сам
    /// восстановится на следующем тике (100 мс).
    /// Не в TTY — просто println.
    pub fn log(&self, line: &str) {
        if self.is_tty {
            // Стереть текущую строку спиннера, вывести лог, перейти на новую строку
            eprint!("\r\x1b[K");
            eprintln!("{}", line);
            let _ = std::io::stderr().flush();
        } else {
            eprintln!("{}", line);
        }
    }

    /// Устаревший алиас — для совместимости с agent.rs без изменений.
    pub fn add_log_line(&self, line: String) {
        self.log(&line);
    }

    /// Сохранить полный вывод последней команды (для /output).
    pub fn set_last_full_output(&self, output: String) {
        *self
            .last_full_output
            .lock()
            .unwrap_or_else(|e| e.into_inner()) = output;
    }

    pub fn get_last_full_output(&self) -> String {
        self.last_full_output
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    /// Начало нового раунда — здесь больше ничего не нужно:
    /// лог уже напечатан, спиннер живёт сам по себе.
    pub fn begin_round(&self) {
        // no-op: лог-строки печатаются сразу через log(), не накапливаются
    }

    /// Остановить спиннер и вывести финальный ответ.
    pub async fn stop(&self, answer: String) {
        self.running.store(false, Ordering::SeqCst);
        // Дать фоновому таску время стереть строку спиннера
        tokio::time::sleep(Duration::from_millis(150)).await;

        if !answer.is_empty() {
            let rendered = render_markdown(&answer);
            println!("{}", rendered);
            let _ = std::io::stdout().flush();
        }
    }
}
