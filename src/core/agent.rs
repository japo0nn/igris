use crate::{
    core::{
        CoreContext,
        llm::ask_llm,
        task::{build_task_object, spawn_save_message},
    },
    db,
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
    let token_limit = context.config.llm.context_token_limit;
    let estimated_tokens = db::estimate_context_tokens(
        &context.connection.lock().unwrap(),
        &session.id.to_string(),
    ).unwrap_or(0);

    if estimated_tokens > token_limit {
        let retention_days = context.config.llm.retention_days;
        let _ = db::trim_old_messages(
            &context.connection.lock().unwrap(),
            &session.id.to_string(),
            retention_days,
        );
    }

    let max_tokens = context.config.llm.max_tokens;
    let mut content = ask_llm(&messages, &context.config, max_tokens).await?;
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

                                let execution = tokio::task::block_in_place(|| skill.execute(method, args));

                                match execution {
                                    Ok(result) => match &result {
                                        SkillOutput::Text(output) => {
                                            let task_object = build_task_object(
                                                &response.message,
                                                skills,
                                                context,
                                                Some(output.clone()),
                                            )?;

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

                                            eprintln!("Assistant: {}: {}", response.message, args);
                                            eprintln!("System: {}", output);

                                            let estimated_tokens = db::estimate_context_tokens(
                                                &context.connection.lock().unwrap(),
                                                &session.id.to_string(),
                                            ).unwrap_or(0);

                                            if estimated_tokens > token_limit {
                                                let retention_days = context.config.llm.retention_days;
                                                let _ = db::trim_old_messages(
                                                    &context.connection.lock().unwrap(),
                                                    &session.id.to_string(),
                                                    retention_days,
                                                );
                                            }

                                            content = ask_llm(&messages, &context.config, max_tokens).await?;

                                            loop {
                                                match serde_json::from_str::<ActionResponse>(
                                                    &content,
                                                ) {
                                                    Ok(value) => {
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

                                                        eprintln!("System: {}", error);
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

                                        eprintln!("System: {}", error);

                                        loop {
                                            match serde_json::from_str::<ActionResponse>(&content) {
                                                Ok(value) => {
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

                                                    eprintln!("System: {}", error);
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

                    if let Some(next) = next_action {
                        response = next;
                    } else {
                        break;
                    }
                }

                break;
            }
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

                eprintln!("System: {}", error);
            }
        }
    }

    Ok(())
}

async fn handle_error(
    error: IgrisError,
    content: String,
    _skills: &Vec<Box<dyn SkillModule>>,
    context: &CoreContext,
    _messages: &mut Vec<AssistantMessage>,
    session: &Session,
) -> Result<String, IgrisError> {
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
