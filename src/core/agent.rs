use std::io::{self, Write};

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

    context.spinner.start("Thinking...".to_string()).await;
    let mut content = match ask_llm(&messages, &context.config).await {
        Ok(c) => c,
        Err(e @ IgrisError::LlmUnavailable(_)) | Err(e @ IgrisError::LlmTimeout(_)) => {
            context
                .spinner
                .stop(format!("[IGRIS ERROR] LLM недоступен: {}", e))
                .await;
            return Err(e);
        }
        Err(e) => {
            return Err(e);
        }
    };

    'outer: loop {
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
            let mut user_interaction_required = false;

            for (idx, action) in response.actions.iter().enumerate() {
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
                                break;
                            }
                        };

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

                        let execution = skill.execute(method, args).await;

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
                                break;
                            }
                        }
                    }
                    Action::PermissionRequest {
                        action,
                        description,
                        risk_level,
                        options,
                    } => {
                        context.spinner.stop(String::new()).await;
                        user_interaction_required = true;
                        let user_choice =
                            prompt_user_permission(action, description, risk_level, options)?;
                        if !user_choice {
                            error_result = Some(IgrisError::PermissionDenied(format!(
                                "User denied action: {}",
                                description
                            )));
                            break;
                        }
                        context.spinner.start("Thinking...".to_string()).await;
                    }
                    Action::PromptUser { message, options } => {
                        context.spinner.stop(String::new()).await;
                        user_interaction_required = true;
                        let user_input = prompt_user_input(message, options)?;
                        if !combined_output.is_empty() {
                            combined_output.push_str("\n---\n");
                        }
                        combined_output
                            .push_str(&format!("[User response to prompt]: {}", user_input));
                        context.spinner.start("Thinking...".to_string()).await;
                    }
                    Action::RequestData {
                        source,
                        query,
                        limit,
                    } => {
                        if source == "memory" {
                            let memory_results = request_memory_data(context, query, *limit)?;
                            if !combined_output.is_empty() {
                                combined_output.push_str("\n---\n");
                            }
                            combined_output.push_str(&memory_results);
                        } else {
                            let sys_info =
                                format!("[System Info Request] source={}, query={}", source, query);
                            if !combined_output.is_empty() {
                                combined_output.push_str("\n---\n");
                            }
                            combined_output.push_str(&sys_info);
                        }
                    }
                    Action::GenerateChunk {
                        module_name,
                        chunk_index,
                        total_chunks,
                        code_chunk,
                        dependencies,
                    } => {
                        // Placeholder for Self-Improvement Engine (Phase 3)
                        let chunk_info = format!(
                            "[CHUNK {}/{}] module={}, code_len={}, deps={:?}",
                            chunk_index,
                            total_chunks,
                            module_name,
                            code_chunk.len(),
                            dependencies
                        );
                        context
                            .spinner
                            .add_log_line(format!("\x1b[2m|   \x1b[33m{}\x1b[0m", chunk_info));
                        if !combined_output.is_empty() {
                            combined_output.push_str("\n---\n");
                        }
                        combined_output.push_str(&chunk_info);
                    }
                    Action::RespondToUser => {
                        context
                            .spinner
                            .add_log_line(format!("\x1b[2m|   \x1b[36mRespond to user...\x1b[0m"));
                    }
                }

                if error_result.is_some() {
                    break;
                }
                if user_interaction_required {
                    break;
                }
            }

            context
                .spinner
                .set_last_full_output(combined_output.clone());

            content = if let Some(err) = error_result {
                // fix_iteration += 1; (removed: value overwritten by response)
                // if fix_iteration >= context.config.execution.fix_iteration_limit {
                //     let final_state = format!(
                //         "[FIX ERROR] Max fix iterations reached ({}). Last task: {}. Error: {}",
                //         context.config.execution.fix_iteration_limit, response.message, err
                //     );
                //     spawn_save_message(
                //         context,
                //         "user".to_string(),
                //         &ActionResponse {
                //             message: final_state.clone(),
                //             is_done: true,
                //             actions: vec![],
                //             iteration,
                //             fix_iteration,
                //             constraints: None,
                //         },
                //         session,
                //     )
                //     .await?;
                //     return Err(IgrisError::MaxFixIterationsExceeded(
                //         context.config.execution.fix_iteration_limit as usize,
                //     ));
                // }
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
                    &context.connection.lock().unwrap_or_else(|e| e.into_inner()),
                    &session.id.to_string(),
                )
                .unwrap_or(0);
                if estimated_tokens > token_limit {
                    let _ = db::trim_old_messages(
                        &context.connection.lock().unwrap_or_else(|e| e.into_inner()),
                        &session.id.to_string(),
                        context.config.llm.retention_days,
                    );
                }

                ask_llm(messages, &context.config).await?
            };

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

fn prompt_user_permission(
    action: &str,
    description: &str,
    risk_level: &str,
    options: &[String],
) -> Result<bool, IgrisError> {
    eprintln!("\n\x1b[1;33m[PERMISSION REQUEST]\x1b[0m");
    eprintln!("  Action: \x1b[36m{}\x1b[0m", action);
    eprintln!("  Description: \x1b[37m{}\x1b[0m", description);
    eprintln!(
        "  Risk level: \x1b[{}m{}\x1b[0m",
        match risk_level {
            "low" => "32",
            "medium" => "33",
            "high" => "31",
            _ => "37",
        },
        risk_level
    );

    for (i, opt) in options.iter().enumerate() {
        eprintln!("  \x1b[34m[{}]\x1b[0m {}", i + 1, opt);
    }
    eprint!("\x1b[1;34m?\x1b[0m Your choice (1-{}): ", options.len());
    io::stderr()
        .flush()
        .map_err(|e| IgrisError::IoError(e.to_string()))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| IgrisError::IoError(e.to_string()))?;
    let input = input.trim().to_lowercase();

    if input == "1" || input.starts_with('y') || input == "yes" || input == "разрешить" {
        eprintln!("\x1b[32m[PERMISSION GRANTED]\x1b[0m");
        Ok(true)
    } else {
        eprintln!("\x1b[31m[PERMISSION DENIED]\x1b[0m");
        Ok(false)
    }
}

fn prompt_user_input(message: &str, options: &[String]) -> Result<String, IgrisError> {
    eprintln!("\n\x1b[1;36m[PROMPT USER]\x1b[0m");
    eprintln!("  \x1b[37m{}\x1b[0m", message);

    if !options.is_empty() {
        for (i, opt) in options.iter().enumerate() {
            eprintln!("  \x1b[34m[{}]\x1b[0m {}", i + 1, opt);
        }
        eprint!("\x1b[1;34m?\x1b[0m Your choice (1-{}): ", options.len());
    } else {
        eprint!("\x1b[1;34m?\x1b[0m Your response: ");
    }
    io::stderr()
        .flush()
        .map_err(|e| IgrisError::IoError(e.to_string()))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| IgrisError::IoError(e.to_string()))?;
    let input = input.trim().to_string();

    if !options.is_empty() {
        if let Ok(num) = input.parse::<usize>() {
            if num >= 1 && num <= options.len() {
                return Ok(options[num - 1].clone());
            }
        }
        // fallback: return raw input
    }

    Ok(input)
}

fn request_memory_data(
    context: &CoreContext,
    query: &str,
    limit: u32,
) -> Result<String, IgrisError> {
    use crate::db::{get_topics, search_messages};

    let connection = context.connection.lock().unwrap_or_else(|e| e.into_inner());
    let mut results = String::new();

    // Search by topics
    let topics = get_topics(&connection)?;
    let matching_topics: Vec<String> = topics
        .into_iter()
        .filter(|t| query.to_lowercase().contains(&t.to_lowercase()))
        .collect();

    if !matching_topics.is_empty() {
        results.push_str(&format!(
            "[Memory] Matching topics: {:?}\n",
            matching_topics
        ));
    }

    // Search messages by keyword
    if let Ok(records) = search_messages(&connection, query, limit as i64) {
        for record in &records {
            results.push_str(&format!(
                "- [{}] {}: {}\n",
                record.timestamp, record.role, record.content
            ));
        }
    }

    if results.is_empty() {
        results = format!("[Memory] No results found for query: {}", query);
    }

    Ok(results)
}

async fn handle_error(
    error: IgrisError,
    content: String,
    skills: &Vec<Box<dyn SkillModule>>,
    context: &CoreContext,
    messages: &mut Vec<AssistantMessage>,
    session: &Session,
    save_raw_content: bool,
) -> Result<String, IgrisError> {
    // Non-recoverable errors: immediately return to user
    if should_abort_on_error(&error) {
        let error_msg = format!(
            "[IGRIS] Non-recoverable error: {}. Task has been stopped.",
            error
        );
        spawn_save_message(
            context,
            "user".to_string(),
            &ActionResponse {
                message: error_msg.clone(),
                is_done: true,
                actions: vec![],
            },
            session,
        )
        .await?;
        let final_response = ActionResponse {
            message: error_msg,
            is_done: true,
            actions: vec![],
        };
        return Ok(serde_json::to_string(&final_response)
            .map_err(|_| IgrisError::ParseError("Serialization failed".to_string()))?);
    }
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

    let new_content = ask_llm(messages, &context.config).await?;

    Ok(new_content)
}

/// Determines if an error should abort the agent loop immediately
/// without asking LLM for a fix.
fn should_abort_on_error(error: &IgrisError) -> bool {
    matches!(
        error,
        IgrisError::LlmUnavailable(_) | IgrisError::LlmTimeout(_) | IgrisError::ConfigError(_) // | IgrisError::MaxIterationsExceeded(_)
                                                                                               // | IgrisError::MaxFixIterationsExceeded(_)
    )
}
