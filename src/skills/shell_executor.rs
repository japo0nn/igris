use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;

use crate::{
    models::metadata::ModuleMetadata,
    skills::{MethodInfo, SkillError, SkillModule, SkillOutput},
};

const COMMAND_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub struct ShellExecutor {
    pub metadata: ModuleMetadata,
    virtual_cwd: Mutex<PathBuf>,
}

impl ShellExecutor {
    pub fn new() -> Self {
        ShellExecutor {
            metadata: ModuleMetadata {
                name: "ShellExecutor".to_string(),
                version: "0.2.0".to_string(),
                _type: crate::models::metadata::ModuleType::Persistent,
                description: "Execute shell commands and run programs".to_string(),
                author: Some("IGRIS".to_string()),
            },
            virtual_cwd: Mutex::new(
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            ),
        }
    }

    fn try_intercept_cd(&self, cmd: &str) -> Option<Result<String, String>> {
        let trimmed = cmd.trim();
        let lower = trimmed.to_lowercase();

        let arg: &str = if lower == "cd" {
            ""
        } else if lower.starts_with("cd ") {
            trimmed[3..].trim()
        } else if lower == "set-location" || lower == "sl" {
            ""
        } else if lower.starts_with("set-location ") {
            trimmed["set-location ".len()..].trim()
        } else if lower.starts_with("sl ") {
            trimmed[3..].trim()
        } else {
            return None;
        };

        let mut cwd = self.virtual_cwd.lock().unwrap_or_else(|e| e.into_inner());

        let target: PathBuf = if arg.is_empty() {
            // bare `cd` / `Set-Location` -> home directory, like a real shell
            match std::env::var("HOME")
                .ok()
                .or_else(|| std::env::var("USERPROFILE").ok())
            {
                Some(h) => PathBuf::from(h),
                None => cwd.clone(),
            }
        } else {
            let p = Path::new(arg);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                cwd.join(p)
            }
        };

        match target.canonicalize() {
            Ok(resolved) if resolved.is_dir() => {
                let display = resolved.display().to_string();
                *cwd = resolved;
                Some(Ok(format!("Changed directory to: {}", display)))
            }
            Ok(resolved) => Some(Err(format!("Not a directory: {}", resolved.display()))),
            Err(e) => Some(Err(format!("cd: cannot access '{}': {}", arg, e))),
        }
    }
}

fn decode_console_output(bytes: &[u8]) -> String {
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    #[cfg(target_os = "windows")]
    {
        let (decoded, _encoding_used, _had_errors) = encoding_rs::IBM866.decode(bytes);
        return decoded.into_owned();
    }
    #[cfg(not(target_os = "windows"))]
    {
        String::from_utf8_lossy(bytes).to_string()
    }
}


async fn kill_process_tree(pid: Option<u32>) {
    let Some(pid) = pid else { return };

    #[cfg(unix)]
    unsafe {

        libc::kill(-(pid as i32), libc::SIGKILL);
    }

    #[cfg(windows)]
    {
        let _ = tokio::process::Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .output()
            .await;
    }
}

#[async_trait]
impl SkillModule for ShellExecutor {
    fn get_metadata(&self) -> &ModuleMetadata {
        &self.metadata
    }

    fn health_check(&self) -> bool {
        true
    }

    async fn execute(&self, method: &str, args: &str) -> Result<SkillOutput, SkillError> {
        if method != "execute_command" {
            return Err(SkillError::InvalidArgs("Method does not exist".to_string()));
        }

        if let Some(result) = self.try_intercept_cd(args) {
            return match result {
                Ok(message) => Ok(SkillOutput::Text(message)),
                Err(message) => Err(SkillError::ExecutionFailed(message)),
            };
        }

        let cwd = self
            .virtual_cwd
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();

        let mut command;

        #[cfg(target_os = "windows")]
        {
            let wrapped = format!(
                "$ErrorActionPreference = 'Stop'; {} | Out-String -Width 500",
                args
            );
            command = tokio::process::Command::new("powershell");
            command.args([
                "-NoProfile",
                "-NonInteractive",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &wrapped,
            ]);
        }

        #[cfg(not(target_os = "windows"))]
        {
            command = tokio::process::Command::new("sh");
            command.args(["-c", args]);
        }

        command.current_dir(&cwd);
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());
        command.stdin(std::process::Stdio::null());

        #[cfg(unix)]
        unsafe {
            command.pre_exec(|| {
                if libc::setpgid(0, 0) != 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }

        #[cfg(windows)]
        {
            const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
            command.creation_flags(CREATE_NEW_PROCESS_GROUP);
        }

        let child = command
            .spawn()
            .map_err(|e| SkillError::ExecutionFailed(format!("Failed to execute command: {}", e)))?;

        let pid = child.id();

        let output = match tokio::time::timeout(COMMAND_TIMEOUT, child.wait_with_output()).await {
            Ok(Ok(output)) => output,
            Ok(Err(e)) => {
                return Err(SkillError::ExecutionFailed(format!(
                    "Failed to wait for command: {}",
                    e
                )));
            }
            Err(_elapsed) => {
                kill_process_tree(pid).await;
                return Err(SkillError::ExecutionFailed(format!(
                    "[TIMEOUT] Command exceeded {}s and was killed: {}",
                    COMMAND_TIMEOUT.as_secs(),
                    args
                )));
            }
        };

        let stdout = decode_console_output(&output.stdout);
        let stderr = decode_console_output(&output.stderr);

        if !output.status.success() {
            return Err(SkillError::ExecutionFailed(format!(
                "Command exited with status {}: {}",
                output.status, stderr
            )));
        }

        let mut result_text = if stdout.is_empty() {
            "Command executed successfully (no output)".to_string()
        } else {
            stdout
        };

        if !stderr.trim().is_empty() {
            result_text.push_str(&format!("\n[STDERR]\n{}", stderr));
        }

        Ok(SkillOutput::Text(result_text))
    }

    fn available_methods(&self) -> Vec<MethodInfo> {
        vec![MethodInfo {
            method: String::from("execute_command"),
            description: String::from("Execute shell commands and programs"),
            args_description: String::from(
                "PowerShell command string on Windows, sh command on Linux/macOS. \
                 Example: Get-ChildItem -Path C:\\Users or ls -la /home. \
                 `cd`/`Set-Location` persists across separate calls (tracked \
                 virtually, since each call is a fresh process). Commands \
                 are killed if they run longer than 60 seconds.",
            ),
        }]
    }
}