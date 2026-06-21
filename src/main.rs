use std::env;
use std::sync::{Arc, Mutex};

use crate::{
    core::{
        CoreContext,
        agent::execute_agent_loop,
        chat::chat_loopback,
        task::{build_task_object, spawn_save_message},
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
        messages.push(AssistantMessage {
            role: "user".to_string(),
            content: serde_json::json!(&task_object).to_string(),
        });

        spawn_save_message(
            &context,
            "user".to_string(),
            &ActionResponse {
                iteration: 0,
                fix_iteration: 0,
                constraints: None,
                message: message.clone(),
                is_done: true,
                actions: vec![],
            },
            &session,
        )
        .await?;
        crate::core::terminal_logger::log_input(&message);

        execute_agent_loop(&mut messages, &context, &skills, &session).await?;
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
        let rx = voice_controller.start_mic()
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
            // 1. STOP the microphone IMMEDIATELY after receiving transcribed text
            voice_controller.stop_mic();
            eprintln!("[Voice] Mic stopped during processing.");

            // 2. Process the transcribed text through the agent
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
            )
            .await?;

            // 3. TTS: speak the last assistant message
            if let Some(last) = session_messages
                .iter()
                .rev()
                .find(|m| m.role == "assistant")
            {
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&last.content) {
                    if let Some(msg) = json_value.get("message").and_then(|v| v.as_str()) {
                        if !msg.is_empty() {
                            // Speak (blocking — waits for TTS to finish)
                            speak_text(msg);
                            voice::trigger_cooldown();
                        }
                    }
                }
            }

            // 4. RESTART the microphone after TTS is fully done
            eprintln!("[Voice] Restarting mic...");
            let _rx_new = voice_controller.restart_mic()
                .expect("Failed to restart voice listener");
            // The loop will now listen on the new receiver, but we need to replace rx.
            // Since while let Ok(text) = rx.recv() owns rx, we break and restart the loop.
            // Instead, we'll use a mutable reference pattern.
            // Actually simpler: just restart and continue with same rx? No, start_mic creates new rx.
            // We need to restructure: use loop with let rx = &mut rx? 
            // Let's just wrap in an outer loop.
            break; // Temporary break to restructure below
        }

        // Restructured: outer loop re-binds rx each time
        // Actually let's rewrite a bit differently - use a while loop with manual control
        eprintln!("[Voice] Entering main voice loop...");
        loop {
            let mut controller = voice::VoiceController::new(&groq_api_key);
            let rx = controller.start_mic()
                .expect("Failed to start voice listener");
            eprintln!("[Voice] Mic started, waiting for voice...");
            
            match rx.recv() {
                Ok(text) => {
                    // Stop mic
                    controller.stop_mic();
                    eprintln!("[Voice] Mic stopped, processing: {}", text);
                    
                    // Process
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
                    ).await?;
                    
                    // TTS
                    if let Some(last) = session_messages.iter().rev().find(|m| m.role == "assistant") {
                        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&last.content) {
                            if let Some(msg) = json_value.get("message").and_then(|v| v.as_str()) {
                                if !msg.is_empty() {
                                    speak_text(msg);
                                    voice::trigger_cooldown();
                                }
                            }
                        }
                    }
                    
                    // Mic will be dropped at end of scope, restart in next iteration
                }
                Err(_) => {
                    eprintln!("[Voice] Listener channel closed, restarting...");
                }
            }
            // Drop controller -> stop mic, loop back to create new one
        }
    } else {
        chat_loopback(&context, &session, &skills, initial_history).await?;
    }

    crate::core::terminal_logger::log_session_end(version);
    crate::core::terminal_logger::log_session_end(version);
    sv.log_event(supervisor::SupervisorEvent::Shutdown);
    Ok(())
}

/// Cross-platform TTS wrapper around core::utils::speak_text.
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
                    content: m.content,
                })
                .collect(),
            Err(_) => vec![],
        },
        _ => vec![],
    }
}
