use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;

use crate::{
    models::metadata::ModuleMetadata,
    skills::{MethodInfo, SkillError, SkillModule, SkillOutput},
};

// ===========================================================================
// Constants
// ===========================================================================

/// Hard wall-clock limit for any single command. After this the whole process
/// tree is force-killed and a `[TIMEOUT]` error is returned.
const COMMAND_TIMEOUT: Duration = Duration::from_secs(60);

/// Upper bound (in chars) on the text returned to the LLM, applied per stream
/// (stdout / stderr). Protects the context window from multi-megabyte dumps.
const MAX_OUTPUT_CHARS: usize = 200_000;

/// Width handed to PowerShell's `Out-String` so wide tables and long lines are
/// not wrapped at the default 80/120 columns when there is no real console.
const PS_OUTPUT_WIDTH: u32 = 4096;

// ===========================================================================
// Struct
// ===========================================================================

#[derive(Debug)]
pub struct ShellExecutor {
    pub metadata: ModuleMetadata,
    /// Every command runs in a brand-new process, so a real `cd` can never
    /// persist between calls. We emulate it: a *standalone* `cd`/`Set-Location`
    /// updates this path, and every following command is spawned with it as the
    /// working directory.
    virtual_cwd: Mutex<PathBuf>,
}

impl ShellExecutor {
    pub fn new() -> Self {
        ShellExecutor {
            metadata: ModuleMetadata {
                name: "ShellExecutor".to_string(),
                version: "0.4.0".to_string(),
                _type: crate::models::metadata::ModuleType::Persistent,
                description: "Execute any shell command or program (cross-platform: \
                    bash/sh on Unix, PowerShell on Windows; UTF-8 safe, timeout + \
                    process-tree kill, virtual cwd)"
                    .to_string(),
                author: Some("IGRIS".to_string()),
            },
            virtual_cwd: Mutex::new(
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            ),
        }
    }

    /// Returns `Some(..)` ONLY when the entire command is a pure, standalone
    /// directory change. Anything that contains shell operators
    /// (`;`, `|`, `&`, `>`, `<`, backtick, newline) is treated as a real command
    /// and left for the shell — this is exactly what used to break on inputs like
    /// `cd "$env:USERPROFILE\Documents" ; Get-Location`.
    fn try_intercept_cd(&self, cmd: &str) -> Option<Result<String, String>> {
        let trimmed = cmd.trim();

        // Compound / piped / redirected -> not a pure cd, let the shell handle it.
        if trimmed.contains(|c: char| {
            matches!(c, ';' | '|' | '&' | '>' | '<' | '`' | '\n' | '\r')
        }) {
            return None;
        }

        let lower = trimmed.to_lowercase();
        let raw_arg: &str = if lower == "cd"
            || lower == "chdir"
            || lower == "set-location"
            || lower == "sl"
        {
            ""
        } else if lower.starts_with("cd ") {
            trimmed[3..].trim()
        } else if lower.starts_with("chdir ") {
            trimmed["chdir ".len()..].trim()
        } else if lower.starts_with("set-location ") {
            trimmed["set-location ".len()..].trim()
        } else if lower.starts_with("sl ") {
            trimmed[3..].trim()
        } else {
            return None;
        };

        // Strip one layer of surrounding quotes, then expand %VAR%, $env:VAR,
        // ${VAR}, $VAR and a leading ~ before touching the filesystem.
        let unquoted = strip_quotes(raw_arg);
        let expanded = expand_vars(unquoted);

        let mut cwd = self.virtual_cwd.lock().unwrap_or_else(|e| e.into_inner());

        let target: PathBuf = if expanded.is_empty() {
            // bare `cd` / `Set-Location` -> home directory, like a real shell.
            match home_dir() {
                Some(h) => h,
                None => cwd.clone(),
            }
        } else {
            let p = Path::new(&expanded);
            if p.is_absolute() {
                p.to_path_buf()
            } else {
                cwd.join(p)
            }
        };

        match target.canonicalize() {
            Ok(resolved) if resolved.is_dir() => {
                let display = strip_unc(&resolved);
                *cwd = resolved;
                Some(Ok(format!("Changed directory to: {}", display)))
            }
            Ok(resolved) => Some(Err(format!("Not a directory: {}", resolved.display()))),
            Err(e) => Some(Err(format!("cd: cannot access '{}': {}", expanded, e))),
        }
    }
}

// ===========================================================================
// Free helpers
// ===========================================================================

/// Pick the best available shell on Unix. We prefer a real `bash` so bash-only
/// syntax (`[[ ]]`, arrays, `<<<`, `<(...)`, `{1..10}`, `${v:0:3}`, heredocs,
/// etc.) behaves identically on Linux and macOS, and only fall back to `/bin/sh`
/// on minimal systems (Alpine/busybox) where bash isn't installed.
#[cfg(not(target_os = "windows"))]
fn pick_unix_shell() -> &'static str {
    const CANDIDATES: [&str; 4] = [
        "/bin/bash",
        "/usr/bin/bash",
        "/usr/local/bin/bash",    // macOS Homebrew (Intel)
        "/opt/homebrew/bin/bash", // macOS Homebrew (Apple Silicon)
    ];
    for p in CANDIDATES {
        if Path::new(p).exists() {
            return p;
        }
    }
    "/bin/sh"
}

/// Decode raw process bytes into a `String`.
///
/// We always *try* strict UTF-8 first (the norm on Unix and, thanks to the UTF-8
/// preamble, from PowerShell on Windows). When a legacy Windows program ignores
/// the console encoding and emits OEM bytes we fall back to cp866 (RU console
/// default), then cp1251 (ANSI Cyrillic), then lossy UTF-8.
fn decode_console_output(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }

    #[cfg(target_os = "windows")]
    {
        let (oem, ..) = encoding_rs::IBM866.decode(bytes);
        if !oem.contains('\u{FFFD}') {
            return oem.into_owned();
        }
        let (ansi, ..) = encoding_rs::WINDOWS_1251.decode(bytes);
        if !ansi.contains('\u{FFFD}') {
            return ansi.into_owned();
        }
        String::from_utf8_lossy(bytes).into_owned()
    }

    #[cfg(not(target_os = "windows"))]
    {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

/// Cap the text we return so a runaway command can't flood the LLM context.
fn truncate_text(s: String) -> String {
    match s.char_indices().nth(MAX_OUTPUT_CHARS) {
        Some((idx, _)) => format!(
            "{}\n... [output truncated at {} chars]",
            &s[..idx], MAX_OUTPUT_CHARS
        ),
        None => s,
    }
}

/// Remove exactly one pair of matching surrounding quotes, if present.
fn strip_quotes(s: &str) -> &str {
    let b = s.as_bytes();
    if b.len() >= 2
        && ((b[0] == b'"' && b[b.len() - 1] == b'"')
            || (b[0] == b'\'' && b[b.len() - 1] == b'\''))
    {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Best-effort environment-variable expansion for the *intercepted* `cd` path.
/// Supports cmd-style `%VAR%`, PowerShell `$env:VAR` / `${env:VAR}`, generic
/// `$VAR` / `${VAR}`, and a leading `~`. Unknown variables expand to "".
fn expand_vars(input: &str) -> String {
    let work = if input == "~" || input.starts_with("~/") || input.starts_with("~\\") {
        match home_dir() {
            Some(h) => format!("{}{}", h.display(), &input[1..]),
            None => input.to_string(),
        }
    } else {
        input.to_string()
    };

    let chars: Vec<char> = work.chars().collect();
    let n = chars.len();
    let mut out = String::with_capacity(work.len());
    let mut i = 0;

    while i < n {
        let c = chars[i];

        // %VAR%  (cmd / batch style)
        if c == '%' {
            if let Some(j) = (i + 1..n).find(|&j| chars[j] == '%') {
                let name: String = chars[i + 1..j].iter().collect();
                if !name.is_empty() {
                    out.push_str(&std::env::var(&name).unwrap_or_default());
                    i = j + 1;
                    continue;
                }
            }
        // $env:VAR, ${env:VAR}, ${VAR}, $VAR
        } else if c == '$' {
            if i + 1 < n && chars[i + 1] == '{' {
                if let Some(j) = (i + 2..n).find(|&j| chars[j] == '}') {
                    let mut name: String = chars[i + 2..j].iter().collect();
                    if let Some(rest) = name.strip_prefix("env:") {
                        name = rest.to_string();
                    }
                    out.push_str(&std::env::var(&name).unwrap_or_default());
                    i = j + 1;
                    continue;
                }
            } else {
                let mut k = i + 1;
                let tail: String = chars[k..].iter().collect();
                if tail.to_lowercase().starts_with("env:") {
                    k += 4; // skip the PowerShell `env:` scope prefix
                }
                let start = k;
                while k < n && (chars[k].is_alphanumeric() || chars[k] == '_') {
                    k += 1;
                }
                if k > start {
                    let name: String = chars[start..k].iter().collect();
                    out.push_str(&std::env::var(&name).unwrap_or_default());
                    i = k;
                    continue;
                }
            }
        }

        out.push(c);
        i += 1;
    }

    out
}

/// Home directory from `HOME` (Unix) or `USERPROFILE` (Windows).
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
}

/// `Path::canonicalize` on Windows yields a `\\?\C:\...` verbatim path; strip the
/// `\\?\` prefix purely for nicer display.
fn strip_unc(p: &Path) -> String {
    let s = p.display().to_string();
    s.strip_prefix(r"\\?\").map(|x| x.to_string()).unwrap_or(s)
}

/// Kill the whole process tree spawned for a command (used on timeout).
async fn kill_process_tree(pid: Option<u32>) {
    let Some(pid) = pid else { return };

    #[cfg(unix)]
    unsafe {
        // The child is its own process-group leader (see pre_exec), so a negative
        // pid signals the whole group, catching grandchildren too.
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

// ===========================================================================
// SkillModule impl
// ===========================================================================

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
        if args.trim().is_empty() {
            return Err(SkillError::InvalidArgs("Empty command".to_string()));
        }

        // 1) Virtual `cd` — handled in-process, never spawns a shell.
        if let Some(result) = self.try_intercept_cd(args) {
            return match result {
                Ok(message) => Ok(SkillOutput::Text(message)),
                Err(message) => Err(SkillError::ExecutionFailed(message)),
            };
        }

        // 2) Snapshot the virtual cwd to spawn the child in.
        let cwd = self
            .virtual_cwd
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();

        // 3) Build the platform-specific command.
        let mut command;

        #[cfg(target_os = "windows")]
        {
            // We deliberately DO NOT set `$ErrorActionPreference='Stop'`: with Stop,
            // a native program's stderr (e.g. cargo's compile progress) is turned
            // into a terminating error and the command "fails" with
            // NativeCommandError even on success. Instead we forward the real exit
            // code via `exit $LASTEXITCODE`.
            //
            // UTF-8 is forced so Cyrillic output is not mojibake. The whole user
            // command runs inside `& { ... }` so multi-statement input (`a; b; c`)
            // is formatted as a single output stream by Out-String — this makes
            // here-strings, file writes (Set-Content/Out-File) and chained commands
            // behave the same as in an interactive console.
            let wrapped = format!(
                "$ProgressPreference='SilentlyContinue'; \
                 [Console]::OutputEncoding=[System.Text.Encoding]::UTF8; \
                 $OutputEncoding=[System.Text.Encoding]::UTF8; \
                 & {{\n{}\n}} | Out-String -Width {}; \
                 exit $LASTEXITCODE",
                args, PS_OUTPUT_WIDTH
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
            // Prefer real bash so bash syntax works identically on Linux and macOS;
            // fall back to /bin/sh on minimal systems. `-c` keeps the full command
            // string intact (pipes, redirects, heredocs, `&&`, subshells, etc.).
            command = tokio::process::Command::new(pick_unix_shell());
            command.args(["-c", args]);
        }

        command.current_dir(&cwd);
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());
        command.stdin(std::process::Stdio::null()); // never block waiting on input
        command.kill_on_drop(true);

        // Put the child in its own process group so a timeout can kill the tree.
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
            const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
            command.creation_flags(CREATE_NEW_PROCESS_GROUP);
        }

        crate::core::terminal_logger::log_shell_command(args);

        let child = command.spawn().map_err(|e| {
            SkillError::ExecutionFailed(format!("Failed to execute command: {}", e))
        })?;
        let pid = child.id();

        let output =
            match tokio::time::timeout(COMMAND_TIMEOUT, child.wait_with_output()).await {
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

        let stdout = truncate_text(decode_console_output(&output.stdout));
        let stderr = truncate_text(decode_console_output(&output.stderr));

        if !output.status.success() {
            let code = output
                .status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "signal".to_string());

            let mut msg = format!("Command exited with status {}", code);

            // Friendly hint for the two most common causes of failure.
            let low = stderr.to_lowercase();
            if low.contains("access denied")
                || low.contains("permission denied")
                || low.contains("requires elevation")
                || low.contains("отказано")
                || low.contains("доступ")
            {
                msg.push_str(
                    " (looks like a permissions problem — on Windows run elevated, \
                     on Unix consider sudo)",
                );
            } else if low.contains("command not found")
                || low.contains("is not recognized")
                || low.contains("no such file or directory")
            {
                msg.push_str(
                    " (program not found — check it is installed and on PATH; \
                     IGRIS inherits the PATH of the process that launched it)",
                );
            }

            if !stdout.trim().is_empty() {
                msg.push_str(&format!("\n[STDOUT]\n{}", stdout.trim()));
            }
            if !stderr.trim().is_empty() {
                msg.push_str(&format!("\n[STDERR]\n{}", stderr.trim()));
            }
            return Err(SkillError::ExecutionFailed(msg));
        }

        // Success: stdout is the answer; stderr (warnings) is appended if present.
        let mut result_text = if stdout.trim().is_empty() {
            "Command executed successfully (no output)".to_string()
        } else {
            stdout
        };
        if !stderr.trim().is_empty() {
            result_text.push_str(&format!("\n[STDERR]\n{}", stderr.trim()));
        }

        crate::core::terminal_logger::log_shell_result(&result_text, false);
        Ok(SkillOutput::Text(result_text))
    }

    fn available_methods(&self) -> Vec<MethodInfo> {
        vec![MethodInfo {
            method: String::from("execute_command"),
            description: String::from(
                "Execute ANY shell command or program and return its combined output \
                 (run programs, build projects, write files, run python/node scripts, \
                 git, package managers, etc.).",
            ),
            args_description: String::from(
                "A single command string.\n\
                 - Windows: run through PowerShell (UTF-8 forced, ExecutionPolicy \
                 Bypass). Use PowerShell syntax, e.g. Set-Content -Path file.py -Value '...', \
                 python script.py, Get-ChildItem C:\\Users, cargo build.\n\
                 - Linux/macOS: run through bash (falls back to sh). Full bash syntax \
                 works: pipes, &&, redirects, heredocs (cat > f.py << 'EOF' ... EOF), \
                 subshells. E.g. python3 script.py, ls -la /home.\n\
                 Pipes/redirects and multi-statement commands (a; b | c) work. Native \
                 exit codes are honored (a successful cargo build is NOT a failure). \
                 To write a file, either redirect/heredoc (bash) or use Set-Content/Out-File \
                 (PowerShell), then run it as a separate or chained command.\n\
                 A STANDALONE cd/Set-Location persists across calls (virtual cwd, since \
                 each call is a fresh process); inside a compound command it only affects \
                 that one invocation - prefer passing paths explicitly (--manifest-path, \
                 -Path). Commands are killed after 60s.",
            ),
        }]
    }
}