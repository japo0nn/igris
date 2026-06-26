use chrono::Local;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct Supervisor {
    pub version: String,
    pub binary_path: PathBuf,
    pub backups_dir: PathBuf,
    pub log_file: PathBuf,
    running: Arc<AtomicBool>,
    pub state: SupervisorState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SupervisorState {
    Running,
    Stopped,
    Restarting,
    Error(String),
}

#[derive(Debug, Clone)]
pub enum SupervisorEvent {
    Startup,
    Shutdown,
    Restart {
        reason: String,
    },
    Error {
        message: String,
    },
    BackupCreated {
        path: String,
    },
    Rollback {
        from_version: String,
        to_version: String,
    },
}

impl Supervisor {
    pub fn new(version: &str) -> Self {
        let binary_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("igris"));
        let backups_dir = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("igris")
            .join("backups");
        let _ = fs::create_dir_all(&backups_dir);
        let log_file = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("igris")
            .join("supervisor.log");

        Supervisor {
            version: version.to_string(),
            binary_path,
            backups_dir,
            log_file,
            running: Arc::new(AtomicBool::new(true)),
            state: SupervisorState::Running,
        }
    }

    pub fn log_event(&self, event: SupervisorEvent) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
        let entry = match &event {
            SupervisorEvent::Startup => {
                format!(
                    "[{}] [STARTUP] IGRIS v{} started from {}\n",
                    timestamp,
                    self.version,
                    self.binary_path.display()
                )
            }
            SupervisorEvent::Shutdown => {
                format!(
                    "[{}] [SHUTDOWN] IGRIS v{} stopped\n",
                    timestamp, self.version
                )
            }
            SupervisorEvent::Restart { reason } => {
                format!(
                    "[{}] [RESTART] IGRIS v{} restarting: {}\n",
                    timestamp, self.version, reason
                )
            }
            SupervisorEvent::Error { message } => {
                format!("[{}] [ERROR] v{}: {}\n", timestamp, self.version, message)
            }
            SupervisorEvent::BackupCreated { path } => {
                format!("[{}] [BACKUP] Created backup at {}\n", timestamp, path)
            }
            SupervisorEvent::Rollback {
                from_version,
                to_version,
            } => {
                format!(
                    "[{}] [ROLLBACK] From v{} to v{}\n",
                    timestamp, from_version, to_version
                )
            }
        };

        // Write to log file
        if let Ok(mut file) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file)
        {
            use std::io::Write;
            let _ = file.write_all(entry.as_bytes());
        }

        eprint!("{}", entry);
    }

    pub fn create_backup(&self) -> Result<String, String> {
        let timestamp = Local::now().format("%Y%m%d_%H%M%S").to_string();
        let backup_name = format!("igris_v{}_{}", self.version, timestamp);
        let backup_path = self.backups_dir.join(&backup_name);

        fs::copy(&self.binary_path, &backup_path)
            .map_err(|e| format!("Failed to create backup: {}", e))?;

        // Keep only last 5 backups
        self.cleanup_old_backups(5);

        Ok(backup_path.to_string_lossy().to_string())
    }

    pub fn rollback(&self, backup_name: &str) -> Result<(), String> {
        let backup_path = self.backups_dir.join(backup_name);
        if !backup_path.exists() {
            return Err(format!("Backup not found: {}", backup_path.display()));
        }

        fs::copy(&backup_path, &self.binary_path).map_err(|e| format!("Rollback failed: {}", e))?;

        self.log_event(SupervisorEvent::Rollback {
            from_version: self.version.clone(),
            to_version: backup_name.to_string(),
        });

        Ok(())
    }

    pub fn restart(&self) -> Result<(), String> {
        self.log_event(SupervisorEvent::Restart {
            reason: "Process restart requested".to_string(),
        });

        // Create backup before restart
        self.create_backup()?;

        // Restart the current process using exec (Unix) or spawn
        let binary = &self.binary_path;
        let args: Vec<String> = std::env::args().collect();

        // On Unix, exec replaces current process
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let err = Command::new(binary).args(&args[1..]).exec();
            return Err(format!("Restart failed (exec error): {}", err));
        }

        #[cfg(not(unix))]
        {
            let _child = Command::new(binary)
                .args(&args[1..])
                .spawn()
                .map_err(|e| format!("Failed to spawn new process: {}", e))?;
            std::process::exit(0);
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    pub fn stop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        self.state = SupervisorState::Stopped;
        self.log_event(SupervisorEvent::Shutdown);
    }

    fn cleanup_old_backups(&self, keep_count: usize) {
        let read_dir = match fs::read_dir(&self.backups_dir) {
            Ok(rd) => rd,
            Err(_) => return,
        };
        let mut entries: Vec<_> = read_dir
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .file_name()
                    .map_or(false, |n| n.to_string_lossy().starts_with("igris_v"))
            })
            .collect();

        entries.sort_by_key(|e| e.path().metadata().ok().map(|m| m.modified().ok()));

        while entries.len() > keep_count {
            if let Some(oldest) = entries.first() {
                let _ = fs::remove_file(oldest.path());
                entries.remove(0);
            }
        }
    }

    pub fn get_backups(&self) -> Vec<String> {
        let read_dir = match fs::read_dir(&self.backups_dir) {
            Ok(rd) => rd,
            Err(_) => return Vec::new(),
        };
        let mut backups: Vec<String> = read_dir
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .file_name()
                    .map_or(false, |n| n.to_string_lossy().starts_with("igris_v"))
            })
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        backups.sort();
        backups
    }
}

// Legacy API for backward compatibility
pub fn log_event(event: SupervisorEvent) {
    // Create a temporary supervisor just for logging
    let supervisor = Supervisor::new("0.1.0");
    supervisor.log_event(event);
}
