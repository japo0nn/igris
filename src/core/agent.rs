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
    let estimated_tokens =
        db::estimate_context_tokens(&context.connection.lock().unwrap(), &session.id.to_string())
            .unwrap_or(0);

    if estimated_tokens > token_limit {
        let retention_days = context.config.llm.retention_days;
        let _ = db::trim_old_messages(
            &context.connection.lock().unwrap(),
            &session.id.to_string(),
            retention_days,
        );
    }

    let max_tokens = context.config.llm.max_tokens;
    context.spinner.start("Thinking...".to_string()).await;
    let mut content = ask_llm(&messages, &context.config, max_tokens).await?;

    'outer: loop {
        // Parse LLM response, retry on parse error
        let mut response = loop {
            match serde_json::from_str::<ActionResponse>(&content) {
                Ok(r) => break r,
                Err(error) => {
                    eprintln!("System parse error: {}", error);
                    content = handle_error(
                        IgrisError::LlmInvalidResponse(error.to_string()),
                        content,
                        skills,
                        context,
                        messages,
                        session,
                        true,
                    )
                    .await?;
                }
            }
        };

        spawn_save_message(context, "assistant".to_string(), &response, session).await?;
        messages.push(AssistantMessage {
            role: String::from("assistant"),
            content: content.clone(),
        });

        loop {
            if response.is_done {
                context.spinner.stop(response.message.clone()).await;
                break 'outer;
            }

            context.spinner.begin_round();
            let total = response.actions.len();
            let mut combined_output = String::new();
            let mut error_result: Option<IgrisError> = None;

            // Execute all actions sequentially, stop on first failure
            'actions: for (idx, action) in response.actions.iter().enumerate() {
                match action {
                    Action::ExecuteModule {
                        module,
                        method,
                        args,
                    } => {
                        let skill = match find_skill(skills, module) {
                            Ok(s) => s,
                            Err(e) => {
                                context.spinner.add_log_line(format!(
                                    "\x1b[2m|   \x1b[31m[{}/{}] module not found: {}\x1b[0m",
                                    idx + 1,
                                    total,
                                    module
                                ));
                                error_result = Some(IgrisError::SkillError(e.to_string()));
                                break 'actions;
                            }
                        };

                        // Log which command is running
                        if total > 1 {
                            context.spinner.add_log_line(format!(
                                "\x1b[2m| [{}/{}] {}\x1b[0m",
                                idx + 1,
                                total,
                                response.message
                            ));
                        } else {
                            context
                                .spinner
                                .add_log_line(format!("\x1b[2m| {}\x1b[0m", response.message));
                        }
                        context
                            .spinner
                            .add_log_line(format!("\x1b[2m|   \x1b[33m{}\x1b[0m", args));

                        let execution = tokio::task::block_in_place(|| skill.execute(method, args));

                        match execution {
                            Ok(result) => {
                                if let SkillOutput::Text(output) = result {
                                    let line_count = output.lines().count();
                                    let byte_count = output.len();
                                    let summary = if output.len() > 300 {
                                        format!(
                                            "↳ {} строк, {} байт (скрыт, /output для просмотра)",
                                            line_count, byte_count
                                        )
                                    } else {
                                        format!("↳ {} строк, {} байт", line_count, byte_count)
                                    };
                                    context.spinner.add_log_line(format!(
                                        "\x1b[2m|   \x1b[32m{}\x1b[0m",
                                        summary
                                    ));
                                    if !combined_output.is_empty() {
                                        combined_output.push_str("\n---\n");
                                    }
                                    combined_output.push_str(&output);
                                }
                            }
                            Err(error) => {
                                // Show which specific command failed
                                context.spinner.add_log_line(format!(
                                    "\x1b[2m|   \x1b[31m[{}/{}] FAILED: {}\x1b[0m",
                                    idx + 1,
                                    total,
                                    error
                                ));
                                error_result = Some(IgrisError::SkillError(format!(
                                    "Command [{}/{}] failed.\nArgs: {}\nError: {}",
                                    idx + 1,
                                    total,
                                    args,
                                    error
                                )));
                                break 'actions;
                            }
                        }
                    }
                    _ => {}
                }
            }

            context
                .spinner
                .set_last_full_output(combined_output.clone());
            // Single LLM round-trip after all actions complete (or fail)
            content = if let Some(err) = error_result {
                handle_error(
                    err,
                    content.clone(),
                    skills,
                    context,
                    messages,
                    session,
                    false,
                )
                .await?
            } else {
                // Use summary for LLM context, full output stored in spinner
                let task_object = build_task_object(
                    &response.message,
                    skills,
                    context,
                    Some(combined_output.clone()),
                )?;
                let user_msg = ActionResponse {
                    message: combined_output.clone(),
                    is_done: true,
                    actions: vec![],
                };
                spawn_save_message(context, "user".to_string(), &user_msg, session).await?;
                messages.push(AssistantMessage {
                    role: String::from("user"),
                    content: serde_json::json!(&task_object).to_string(),
                });

                let estimated_tokens = db::estimate_context_tokens(
                    &context.connection.lock().unwrap(),
                    &session.id.to_string(),
                )
                .unwrap_or(0);
                if estimated_tokens > token_limit {
                    let _ = db::trim_old_messages(
                        &context.connection.lock().unwrap(),
                        &session.id.to_string(),
                        context.config.llm.retention_days,
                    );
                }

                ask_llm(messages, &context.config, max_tokens).await?
            };

            // Parse next LLM response, retry on parse error
            response = loop {
                match serde_json::from_str::<ActionResponse>(&content) {
                    Ok(value) => {
                        spawn_save_message(context, "assistant".to_string(), &value, session)
                            .await?;
                        messages.push(AssistantMessage {
                            role: String::from("assistant"),
                            content: content.clone(),
                        });
                        break value;
                    }
                    Err(error) => {
                        eprintln!("System parse error: {}", error);
                        content = handle_error(
                            IgrisError::LlmInvalidResponse(error.to_string()),
                            content,
                            skills,
                            context,
                            messages,
                            session,
                            true,
                        )
                        .await?;
                    }
                }
            };
        }
    }

    Ok(())
}

/// Handles errors in the agent loop.
///
/// - `save_raw_content`: if true, the raw `content` string (bad/unparsed LLM response)
///   is saved to DB as an assistant message and pushed to `messages`.
///   Set to `true` when content was never saved (e.g. parse error on fresh LLM output).
///   Set to `false` when content was already saved (e.g. skill execution error after
///   a successfully parsed response).
///
/// After saving, pushes the error as a user message to `messages` and calls
/// `ask_llm` to get a fresh regenerated response.
async fn handle_error(
    error: IgrisError,
    content: String,
    skills: &Vec<Box<dyn SkillModule>>,
    context: &CoreContext,
    messages: &mut Vec<AssistantMessage>,
    session: &Session,
    save_raw_content: bool,
) -> Result<String, IgrisError> {
    if save_raw_content {
        messages.push(AssistantMessage {
            role: String::from("assistant"),
            content: content.clone(),
        });
        spawn_save_message(
            context,
            "assistant".to_string(),
            &ActionResponse {
                message: content.clone(),
                is_done: false,
                actions: vec![],
            },
            session,
        )
        .await?;
    }

    let error_str = format!("[SYSTEM EXECUTION RESULT] {}", error);

    spawn_save_message(
        context,
        "user".to_string(),
        &ActionResponse {
            message: error_str.clone(),
            is_done: true,
            actions: vec![],
        },
        session,
    )
    .await?;

    let task_object = build_task_object(&error_str, skills, context, Some(error_str.clone()))?;

    messages.push(AssistantMessage {
        role: String::from("user"),
        content: serde_json::json!(&task_object).to_string(),
    });

    let max_tokens = context.config.llm.max_tokens;
    let new_content = ask_llm(messages, &context.config, max_tokens).await?;

    Ok(new_content)
}
