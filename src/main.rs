use std::env;
use std::sync::{Arc, Mutex};

use crate::{
    core::{
        agent::execute_agent_loop,
        chat::chat_loopback,
        self_improvement::SelfImprovementEngine,
        task::{build_task_object, spawn_save_message_with_raw},
        CoreContext,
    },
    db::{create_session, get_last_session_with_messages, get_messages_by_session, init_database},
    models::assistant::{ActionResponse, AssistantMessage},
    registry::init_modules_metadata,
};

pub mod configs;
pub mod core;
pub mod db;
pub mod error;
pub mod memory;
pub mod models;
pub mod registry;
pub mod skills;
pub mod supervisor;
pub mod token_counter;
pub mod voice;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let name = "IGRIS";
    let version = "0.1.2";
    let status = "Active";

    println!("{name} {version}\nStatus: {status}");

    let sv = supervisor::Supervisor::new(version);
    sv.log_event(supervisor::SupervisorEvent::Startup);
    crate::core::terminal_logger::log_session_start(version);

    let (config, secrets) = configs::llm::load_config()?;

    let connection = init_database(&config)?;

    let spinner = crate::core::spinner::Spinner::new();
    let context = CoreContext {
        connection: Arc::new(Mutex::new(connection)),
        config: config,
        spinner,
        supervisor: sv.clone(),
    };

    let skills = init_modules_metadata(&context)?;
    let session = create_session(&context.connection.lock().unwrap_or_else(|e| e.into_inner()))?;

    // Создаём SelfImprovementEngine для обработки GenerateChunk
    let self_improvement = SelfImprovementEngine::new();

    let initial_history = load_previous_session_history(&context);
    if !initial_history.is_empty() {
        eprintln!(
            "[IGRIS] Loaded {} messages from previous session.",
            initial_history.len()
        );
    }

    let args: Vec<String> = env::args().collect();
    if args.len() >= 3 && (args[1] == "--message" || args[1] == "-m") {
        let message = args[2..].join(" ");

        let mut messages: Vec<AssistantMessage> = vec![AssistantMessage {
            role: String::from("system"),
            content: context.config.llm.system_prompt.clone(),
        }];

        for msg in initial_history {
            if msg.role != "system" {
                messages.push(msg);
            }
        }

        let task_object = build_task_object(&message, &skills, &context, None)?;
        let task_obj_json = serde_json::json!(&task_object).to_string();
        messages.push(AssistantMessage {
            role: "user".to_string(),
            content: task_obj_json.clone(),
        });

        spawn_save_message_with_raw(
            &context,
            "user".to_string(),
            &ActionResponse {
                message: message.clone(),
                is_done: true,
                actions: vec![],
            },
            Some(&task_obj_json),
            &session,
        )
        .await?;
        crate::core::terminal_logger::log_input(&message);

        execute_agent_loop(
            &mut messages,
            &context,
            &skills,
            &session,
            &self_improvement,
        )
        .await?;
    } else if args.len() == 2 && (args[1] == "--help" || args[1] == "-h") {
        println!("IGRIS v0.1.0");
        println!("Usage:");
        println!("  igris                     - interactive mode (default)");
        println!("  igris --message <text>    - process a single message and exit");
        println!("  igris -m <text>           - same as --message");
        println!("  igris --help              - show this help");
        return Ok(());
    } else if args.len() >= 2 && (args[1] == "--voice" || args[1] == "-v") {
        // Voice mode with full mic lifecycle control
        let groq_api_key = secrets
            .voice
            .as_ref()
            .map(|v| v.groq_api_key.clone())
            .unwrap_or_else(|| {
                eprintln!("[IGRIS] No [voice.groq_api_key] found in secrets.toml");
                std::process::exit(1);
            });

        let mut voice_controller = voice::VoiceController::new(&groq_api_key);
        let rx = voice_controller
            .start_mic()
            .expect("Failed to start voice listener");

        let mut messages = vec![AssistantMessage {
            role: "system".to_string(),
            content: context.config.llm.system_prompt.clone(),
        }];
        for msg in initial_history {
            if msg.role != "system" {
                messages.push(msg);
            }
        }

        eprintln!("[Voice] Listening... Press Ctrl+C to exit.");

        while let Ok(text) = rx.recv() {
            voice_controller.stop_mic();
            eprintln!("[Voice] Mic stopped during processing.");

            let task_object = crate::core::task::build_task_object(&text, &skills, &context, None)?;
            let mut session_messages = messages.clone();
            session_messages.push(AssistantMessage {
                role: "user".to_string(),
                content: serde_json::json!(&task_object).to_string(),
            });

            crate::core::terminal_logger::log_input(&text);
            crate::core::agent::execute_agent_loop(
                &mut session_messages,
                &context,
                &skills,
                &session,
                &self_improvement,
            )
            .await?;

            if let Some(last) = session_messages
                .iter()
                .rev()
                .find(|m| m.role == "assistant")
            {
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&last.content) {
                    if let Some(msg) = json_value.get("message").and_then(|v| v.as_str()) {
                        if !msg.is_empty() {
                            speak_text(msg);
                            voice::trigger_cooldown();
                        }
                    }
                }
            }

            eprintln!("[Voice] Restarting mic...");
            let _rx_new = voice_controller
                .restart_mic()
                .expect("Failed to restart voice listener");
            break;
        }

        eprintln!("[Voice] Entering main voice loop...");
        loop {
            let mut controller = voice::VoiceController::new(&groq_api_key);
            let rx = controller
                .start_mic()
                .expect("Failed to start voice listener");
            eprintln!("[Voice] Mic started, waiting for voice...");

            match rx.recv() {
                Ok(text) => {
                    controller.stop_mic();
                    eprintln!("[Voice] Mic stopped, processing: {}", text);

                    let task_object =
                        crate::core::task::build_task_object(&text, &skills, &context, None)?;
                    let mut session_messages = messages.clone();
                    session_messages.push(AssistantMessage {
                        role: "user".to_string(),
                        content: serde_json::json!(&task_object).to_string(),
                    });
                    crate::core::terminal_logger::log_input(&text);
                    crate::core::agent::execute_agent_loop(
                        &mut session_messages,
                        &context,
                        &skills,
                        &session,
                        &self_improvement,
                    )
                    .await?;

                    if let Some(last) = session_messages
                        .iter()
                        .rev()
                        .find(|m| m.role == "assistant")
                    {
                        if let Ok(json_value) =
                            serde_json::from_str::<serde_json::Value>(&last.content)
                        {
                            if let Some(msg) = json_value.get("message").and_then(|v| v.as_str()) {
                                if !msg.is_empty() {
                                    speak_text(msg);
                                    voice::trigger_cooldown();
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    eprintln!("[Voice] Listener channel closed, restarting...");
                }
            }
        }
    } else {
        chat_loopback(&context, &session, &skills, initial_history).await?;
    }

    crate::core::terminal_logger::log_session_end(version);
    sv.log_event(supervisor::SupervisorEvent::Shutdown);
    Ok(())
}

fn speak_text(text: &str) {
    let _ = crate::core::utils::speak_text(text);
}

fn load_previous_session_history(context: &CoreContext) -> Vec<AssistantMessage> {
    let connection = context.connection.lock().unwrap_or_else(|e| e.into_inner());
    match get_last_session_with_messages(&connection) {
        Ok(Some(last_session)) => match get_messages_by_session(&connection, &last_session.id) {
            Ok(messages) => messages
                .into_iter()
                .map(|m| AssistantMessage {
                    role: m.role,
                    content: m.raw_json.clone().unwrap_or(m.content.clone()),
                })
                .collect(),
            Err(_) => vec![],
        },
        _ => vec![],
    }
}
