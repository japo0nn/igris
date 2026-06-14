use std::borrow::Cow;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;

use rustyline::completion::{Completer, Pair};
use rustyline::config::Builder;
use rustyline::error::ReadlineError;
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::{ValidationContext, ValidationResult, Validator};
use rustyline::{Context, Editor, Helper, history::FileHistory};

use crate::{
    core::{
        CoreContext,
        agent::execute_agent_loop,
        task::{build_task_object, spawn_save_message},
    },
    error::IgrisError,
    memory::Session,
    models::assistant::{ActionResponse, AssistantMessage},
    skills::SkillModule,
};

const PROMPT: &str = "\x1b[1;34m\u{276f}\x1b[0m ";

pub async fn chat_loopback(
    context: &CoreContext,
    session: &Session,
    skills: &Vec<Box<dyn SkillModule>>,
    initial_history: Vec<AssistantMessage>,
) -> Result<(), IgrisError> {
    let mut messages: Vec<AssistantMessage> = vec![AssistantMessage {
        role: String::from("system"),
        content: context.config.llm.system_prompt.clone(),
    }];

    let window_size = 15;
    let history_to_load = if initial_history.len() > window_size {
        initial_history[initial_history.len() - window_size..].to_vec()
    } else {
        initial_history
    };
    for msg in history_to_load {
        if msg.role != "system" {
            messages.push(msg);
        }
    }

    let config = Builder::new().auto_add_history(true).build();
    let mut rl = Editor::with_config(config)?;
    rl.set_helper(Some(IgrisHelper));

    let history_path = get_history_path();
    let _ = rl.load_history(&history_path);

    println!(
        "\x1b[1;32mIGRIS\x1b[0m \x1b[1;36mv0.1.0\x1b[0m — interactive mode. Type \x1b[33m/help\x1b[0m for commands."
    );

    loop {
        let readline = rl.readline(PROMPT);
        match readline {
            Ok(line) => {
                let trimmed = line.trim().to_string();
                if trimmed.is_empty() {
                    continue;
                }

                if trimmed == "/exit" || trimmed == "/q" || trimmed == "exit" {
                    println!("Goodbye!");
                    break;
                }

                if trimmed.starts_with('/') {
                    let msg = handle_slash_command(
                        &trimmed,
                        &mut rl,
                        &history_path,
                        context,
                        session,
                        skills,
                        &mut messages,
                    )?;
                    if let Some(m) = msg {
                        process_user_input(
                            m,
                            &mut rl,
                            &history_path,
                            context,
                            session,
                            skills,
                            &mut messages,
                        )
                        .await?;
                    }
                    continue;
                }

                process_user_input(
                    trimmed,
                    &mut rl,
                    &history_path,
                    context,
                    session,
                    skills,
                    &mut messages,
                )
                .await?;
            }
            Err(ReadlineError::Interrupted) | Err(ReadlineError::Eof) => {
                println!();
                break;
            }
            Err(err) => {
                eprintln!("\x1b[31m[IGRIS] Input error: {}\x1b[0m", err);
                break;
            }
        }

        if let Err(e) = rl.save_history(&history_path) {
            eprintln!(
                "\x1b[33m[IGRIS] Warning: failed to save history: {}\x1b[0m",
                e
            );
        }
    }

    Ok(())
}

fn get_history_path() -> PathBuf {
    let mut path = if let Ok(dir) = env::var("XDG_DATA_HOME") {
        PathBuf::from(dir)
    } else if let Ok(home) = env::var("HOME") {
        PathBuf::from(home).join(".local").join("share")
    } else {
        PathBuf::from(".")
    };
    let _ = fs::create_dir_all(&path);
    path.push("igris_history.txt");
    path
}

fn handle_slash_command(
    cmd: &str,
    rl: &mut Editor<IgrisHelper, FileHistory>,
    _history_path: &PathBuf,
    _context: &CoreContext,
    _session: &Session,
    _skills: &Vec<Box<dyn SkillModule>>,
    _messages: &mut Vec<AssistantMessage>,
) -> Result<Option<String>, IgrisError> {
    match cmd {
        "/help" | "/h" => {
            println!(
                "\x1b[1;34m... Commands ...\x1b[0m\n\
                 \x1b[33m/help\x1b[0m, \x1b[33m/h\x1b[0m      — Show this help\n\
                 \x1b[33m/clear\x1b[0m, \x1b[33m/c\x1b[0m     — Clear the screen\n\
                 \x1b[33m/exit\x1b[0m, \x1b[33m/q\x1b[0m      — Exit IGRIS\n\
                 \x1b[33m/history\x1b[0m         — Show recent commands\n\
                 \x1b[33m/edit\x1b[0m, \x1b[33m/e\x1b[0m      — Open external editor for long input"
            );
            Ok(None)
        }
        "/clear" | "/c" => {
            print!("\x1b[2J\x1b[1;1H");
            let _ = io::stdout().flush();
            Ok(None)
        }
        "/history" => {
            let entries: Vec<_> = rl.history().iter().collect();
            if entries.is_empty() {
                println!("(empty)");
            } else {
                let start = if entries.len() > 20 {
                    entries.len() - 20
                } else {
                    0
                };
                for (i, entry) in entries[start..].iter().enumerate() {
                    let display = if entry.len() > 80 {
                        format!("{}...", &entry[..80])
                    } else {
                        entry.to_string()
                    };
                    println!("  \x1b[33m{}\x1b[0m: {}", start + i + 1, display);
                }
            }
            Ok(None)
        }
        cmd if cmd.starts_with("/edit") || cmd.starts_with("/e ") || cmd == "/e" => {
            let initial = if cmd.starts_with("/edit") {
                cmd[5..].trim().to_string()
            } else if cmd == "/e" {
                String::new()
            } else {
                cmd[2..].trim().to_string()
            };
            let content = open_editor(initial)?;
            if content.is_empty() {
                println!("\x1b[2m(empty input, ignored)\x1b[0m");
                Ok(None)
            } else {
                Ok(Some(content))
            }
        }
        _ => {
            println!(
                "\x1b[31mUnknown command:\x1b[0m \x1b[33m{}\x1b[0m. Type \x1b[33m/help\x1b[0m for available commands.",
                cmd
            );
            Ok(None)
        }
    }
}

fn open_editor(initial: String) -> Result<String, IgrisError> {
    let editor = env::var("EDITOR")
        .or_else(|_| env::var("VISUAL"))
        .unwrap_or_else(|_| "vim".to_string());

    let mut tmp = env::temp_dir();
    tmp.push(format!("igris_edit_{}.md", std::process::id()));

    fs::write(&tmp, &initial)
        .map_err(|e| IgrisError::IoError(format!("Cannot write temp file: {}", e)))?;

    let status = Command::new(&editor)
        .arg(&tmp)
        .status()
        .map_err(|e| IgrisError::IoError(format!("Cannot launch editor '{}': {}", editor, e)))?;

    if !status.success() {
        return Err(IgrisError::IoError(format!(
            "Editor '{}' exited with status {}",
            editor,
            status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        )));
    }

    let content = fs::read_to_string(&tmp)
        .map_err(|e| IgrisError::IoError(format!("Cannot read edited file: {}", e)))?;

    let _ = fs::remove_file(&tmp);
    Ok(content.trim().to_string())
}

async fn process_user_input(
    input: String,
    rl: &mut Editor<IgrisHelper, FileHistory>,
    history_path: &PathBuf,
    context: &CoreContext,
    session: &Session,
    skills: &Vec<Box<dyn SkillModule>>,
    messages: &mut Vec<AssistantMessage>,
) -> Result<(), IgrisError> {
    let task_object = build_task_object(&input, skills, context, None)?;
    messages.push(AssistantMessage {
        role: "user".to_string(),
        content: serde_json::json!(&task_object).to_string(),
    });

    spawn_save_message(
        context,
        "user".to_string(),
        &ActionResponse {
            message: input,
            is_done: true,
            actions: vec![],
            iteration: 0,
            fix_iteration: 0,
            constraints: None,
        },
        session,
    )
    .await?;

    execute_agent_loop(&mut *messages, context, skills, session).await?;

    if let Err(e) = rl.save_history(history_path) {
        eprintln!(
            "\x1b[33m[IGRIS] Warning: failed to save history: {}\x1b[0m",
            e
        );
    }

    Ok(())
}

struct IgrisHelper;

impl Helper for IgrisHelper {}

impl Completer for IgrisHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        _pos: usize,
        _ctx: &Context<'_>,
    ) -> Result<(usize, Vec<Pair>), ReadlineError> {
        let commands = [
            "/help", "/h", "/clear", "/c", "/exit", "/q", "/history", "/edit", "/e", "/output",
            "/o",
        ];
        let candidates: Vec<Pair> = commands
            .iter()
            .filter(|c| c.starts_with(line))
            .map(|c| Pair {
                display: c.to_string(),
                replacement: c[line.len()..].to_string(),
            })
            .collect();
        if line.starts_with('/') {
            Ok((0, candidates))
        } else {
            Ok((_pos, vec![]))
        }
    }
}

impl Hinter for IgrisHelper {
    type Hint = String;

    fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> {
        None
    }
}

impl Highlighter for IgrisHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        if line.starts_with('/') {
            Cow::Owned(format!("\x1b[33m{}\x1b[0m", line))
        } else {
            Cow::Borrowed(line)
        }
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _forced: bool) -> bool {
        false
    }
}

impl Validator for IgrisHelper {
    fn validate(&self, _ctx: &mut ValidationContext<'_>) -> rustyline::Result<ValidationResult> {
        Ok(ValidationResult::Valid(None))
    }
}
