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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let name = "IGRIS";
    let version = "v0.1.0";
    let status = "Active";

    println!("{name} {version}\nStatus: {status}");

    let (config, _secrets) = configs::llm::load_config()?;

    let connection = init_database(&config)?;

    let context = CoreContext {
        connection: Arc::new(Mutex::new(connection)),
        config: config,
    };

    let skills = init_modules_metadata(&context)?;
    let session = create_session(&context.connection.lock().unwrap())?;

    let initial_history = load_previous_session_history(&context);
    if !initial_history.is_empty() {
        println!(
            "Loaded {} messages from previous session.",
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
                message: message,
                is_done: true,
                actions: vec![],
            },
            &session,
        )
        .await?;

        execute_agent_loop(&mut messages, &context, &skills, &session).await?;
        println!("");
    } else if args.len() == 2 && (args[1] == "--help" || args[1] == "-h") {
        println!("IGRIS v0.1.0");
        println!("Usage:");
        println!("  igris                     - interactive mode (default)");
        println!("  igris --message <text>    - process a single message and exit");
        println!("  igris -m <text>           - same as --message");
        println!("  igris --help              - show this help");
    } else {
        chat_loopback(&context, &session, &skills, initial_history).await?;
    }

    Ok(())
}

fn load_previous_session_history(context: &CoreContext) -> Vec<AssistantMessage> {
    let connection = context.connection.lock().unwrap();
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
