use crate::{
    core::{
        CoreContext,
        llm::ask_llm,
        task::{build_task_object, spawn_save_message},
    },
    error::IgrisError,
    memory::Session,
    models::assistant::{Action, ActionResponse, AssistantMessage},
    skills::{SkillModule, SkillOutput, find_skill},
};

pub async fn execute_agent_loop(
    messages: &mut Vec<AssistantMessage>,
    context: &CoreContext,
    skills: &Vec<Box<dyn SkillModule>>,
    session: &Session,
) -> Result<(), IgrisError> {
    let mut content = ask_llm(&messages, &context.config).await?;
    loop {
        match serde_json::from_str::<ActionResponse>(&content) {
            Ok(mut response) => {
                spawn_save_message(&context, "assistant".to_string(), &response, session).await?;

                messages.push(AssistantMessage {
                    role: String::from("assistant"),
                    content: String::from(&content),
                });

                loop {
                    if response.is_done {
                        break;
                    }

                    let mut next_action: Option<ActionResponse> = None;

                    for action in &response.actions {
                        match action {
                            Action::ExecuteModule {
                                module,
                                method,
                                args,
                            } => {
                                let skill = find_skill(skills, module)?;

                                let execution = skill.execute(method, args);

                                match execution {
                                    Ok(result) => match &result {
                                        SkillOutput::Text(output) => {
                                            let task_object = build_task_object(
                                                &response.message,
                                                skills,
                                                context,
                                                Some(output.clone()),
                                            )?;

                                            // Save the user message (task object) to DB
                                            let user_msg = ActionResponse {
                                                message: format!(
                                                    "[SYSTEM EXECUTION RESULT] {}",
                                                    output
                                                ),
                                                is_done: true,
                                                actions: vec![],
                                            };
                                            spawn_save_message(
                                                &context,
                                                "user".to_string(),
                                                &user_msg,
                                                session,
                                            )
                                            .await?;

                                            messages.push(AssistantMessage {
                                                role: String::from("user"),
                                                content: serde_json::json!(&task_object)
                                                    .to_string(),
                                            });

                                            println!("Assistant: {}: {}", response.message, args);
                                            println!("System: {}", output);

                                            content = ask_llm(&messages, &context.config).await?;

                                            loop {
                                                match serde_json::from_str::<ActionResponse>(
                                                    &content,
                                                ) {
                                                    Ok(value) => {
                                                        // Save the assistant response to DB
                                                        spawn_save_message(
                                                            &context,
                                                            "assistant".to_string(),
                                                            &value,
                                                            session,
                                                        )
                                                        .await?;

                                                        messages.push(AssistantMessage {
                                                            role: String::from("assistant"),
                                                            content: String::from(&content),
                                                        });
                                                        next_action = Some(value);
                                                        break;
                                                    }
                                                    Err(error) => {
                                                        content = handle_error(
                                                            IgrisError::SkillError(
                                                                error.to_string(),
                                                            ),
                                                            content,
                                                            skills,
                                                            &context,
                                                            messages,
                                                            &session,
                                                        )
                                                        .await?;

                                                        println!("System: {}", error);
                                                    }
                                                }
                                            }

                                            if next_action.is_some() {
                                                break;
                                            }
                                        }
                                        _ => {}
                                    },
                                    Err(error) => {
                                        content = handle_error(
                                            IgrisError::SkillError(error.to_string()),
                                            content,
                                            skills,
                                            &context,
                                            messages,
                                            &session,
                                        )
                                        .await?;

                                        println!("System: {}", error);

                                        loop {
                                            match serde_json::from_str::<ActionResponse>(&content) {
                                                Ok(value) => {
                                                    // Save the assistant response to DB
                                                    spawn_save_message(
                                                        &context,
                                                        "assistant".to_string(),
                                                        &value,
                                                        session,
                                                    )
                                                    .await?;

                                                    messages.push(AssistantMessage {
                                                        role: String::from("assistant"),
                                                        content: String::from(&content),
                                                    });
                                                    next_action = Some(value);
                                                    break;
                                                }
                                                Err(error) => {
                                                    content = handle_error(
                                                        IgrisError::ParseError(error.to_string()),
                                                        content,
                                                        skills,
                                                        &context,
                                                        messages,
                                                        &session,
                                                    )
                                                    .await?;

                                                    println!("System: {}", error);
                                                }
                                            }
                                        }

                                        if next_action.is_some() {
                                            break;
                                        }
                                    }
                                }
                            }
                            _ => {}
                        }
                    }

                    if let Some(new_action) = next_action {
                        response = new_action;
                    } else {
                        break;
                    }
                }

                // Save the final assistant response if not already saved (final is_done response)
                if !response.is_done {
                    // This case shouldn't happen after loop break with is_done, but just in case
                    spawn_save_message(&context, "assistant".to_string(), &response, session)
                        .await?;
                }

                println!("Assistant: {}", response.message);

                break;
            }
            Err(error) => {
                content = handle_error(
                    IgrisError::ParseError(error.to_string()),
                    content,
                    skills,
                    &context,
                    messages,
                    &session,
                )
                .await?;

                println!("System: {}", error);
            }
        }
    }

    Ok(())
}

async fn handle_error(
    error: IgrisError,
    mut content: String,
    skills: &Vec<Box<dyn SkillModule>>,
    context: &CoreContext,
    messages: &mut Vec<AssistantMessage>,
    session: &Session,
) -> Result<String, IgrisError> {
    let task_object = build_task_object(&content, skills, context, Some(error.to_string()))?;

    messages.push(AssistantMessage {
        role: String::from("user"),
        content: serde_json::json!(&task_object).to_string(),
    });

    content = ask_llm(&messages, &context.config).await?;

    spawn_save_message(
        &context,
        "user".to_string(),
        &ActionResponse {
            message: format!("[SYSTEM EXECUTION RESULT] {}", error),
            is_done: true,
            actions: vec![],
        },
        session,
    )
    .await?;

    Ok(content)
}
