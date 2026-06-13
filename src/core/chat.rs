use std::io::{self, Write};

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

    // SLIDING WINDOW: Take only the last 15 messages from initial history
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

    loop {
        let input = read_user_input()?;

        match input {
            Some(message) => {
                let task_object = build_task_object(&message, skills, context, None)?;
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
                    session,
                )
                .await?;

                execute_agent_loop(&mut messages, context, skills, session).await?;
            }
            None => return Ok(()),
        }
    }
}

fn read_user_input() -> Result<Option<String>, IgrisError> {
    print!("\n\nYou: ");
    io::stdout().flush()?;
    let stdin = io::stdin();
    let mut input = String::new();
    stdin.read_line(&mut input)?;

    if input.trim() == "exit" {
        return Ok(None);
    }

    Ok(Some(input))
}
