use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};

use crate::{configs::llm::AppConfig, error::IgrisError, models::assistant::AssistantMessage};

pub async fn ask_llm(
    messages: &Vec<AssistantMessage>,
    config: &AppConfig,
    _max_tokens: u32,
) -> Result<String, IgrisError> {
    let max_retries = config.llm.retry_max_retries;
    let initial_delay = config.llm.retry_initial_delay_ms;
    let mut last_error: Option<IgrisError> = None;

    for attempt in 0..max_retries {
        let client = reqwest::Client::new();

        let request_body = serde_json::json!({
            "model": &config.llm.model,
            "messages": messages,
            "stream": false,
        })
        .to_string();

        match client
            .post(format!("{}/v1/chat/completions", config.llm.base_uri))
            .header(
                AUTHORIZATION,
                format!("Bearer {}", config.llm.api_key.as_deref().unwrap_or("")),
            )
            .header(CONTENT_TYPE, "application/json")
            .body(request_body)
            .send()
            .await
        {
            Ok(response) => {
                match response.text().await {
                    Ok(text) => {
                        return Ok(extract_content(&text)?);
                    }
                    Err(e) => {
                        last_error = Some(IgrisError::LlmUnavailable(format!("Response read failed: {}", e)));
                    }
                }
            }
            Err(e) => {
                if e.is_timeout() {
                    last_error = Some(IgrisError::LlmTimeout(e.to_string()));
                } else if e.is_connect() || e.is_request() {
                    last_error = Some(IgrisError::LlmUnavailable(e.to_string()));
                } else {
                    last_error = Some(IgrisError::LlmUnavailable(e.to_string()));
                }
            }
        }

        if attempt < max_retries - 1 {
            let delay = std::time::Duration::from_millis(initial_delay * (attempt as u64 + 1));
            tokio::time::sleep(delay).await;
        }
    }

    Err(last_error.unwrap_or(IgrisError::LlmUnavailable("All retries exhausted".to_string())))
}

pub async fn generate_topics(
    message: String,
    config: &AppConfig,
    max_tokens: u32,
) -> Result<String, IgrisError> {
    let client = reqwest::Client::new();

    let messages: Vec<AssistantMessage> = vec![
        AssistantMessage {
            role: "system".to_string(),
            content: config.topic_llm.system_prompt.clone(),
        },
        AssistantMessage {
            role: "user".to_string(),
            content: message,
        },
    ];

    let request_body = serde_json::json!({
        "model": &config.topic_llm.model,
        "messages": messages,
        "stream": false,
        "max_tokens": max_tokens,
        "thinking": {
            "type": "disabled"
        }
    })
    .to_string();

    let response = client
        .post(format!("{}/v1/chat/completions", config.llm.base_uri))
        .header(
            AUTHORIZATION,
            format!("Bearer {}", config.llm.api_key.as_deref().unwrap_or("")),
        )
        .header(CONTENT_TYPE, "application/json")
        .body(request_body)
        .send()
        .await?
        .text()
        .await?;

    return Ok(extract_content(&response)?);
}

fn extract_content(response: &str) -> Result<String, IgrisError> {
    let raw: serde_json::Value = serde_json::from_str(response)?;

    let content = raw["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or_default()
        .to_string();

    let stripped = remove_markdown_wrapper(&content);
    Ok(sanitize_json_strings(&stripped))
}

fn sanitize_json_strings(json: &str) -> String {
    let mut result = String::with_capacity(json.len() * 2);
    let mut in_string = false;
    let mut chars = json.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '\\' if in_string => {
                match chars.peek() {
                    Some(&'"') | Some(&'\\') | Some(&'/') | Some(&'b') | Some(&'f')
                    | Some(&'n') | Some(&'r') | Some(&'t') => {
                        result.push('\\');
                        result.push(chars.next().unwrap());
                    }
                    Some(&'u') => {
                        result.push('\\');
                        result.push(chars.next().unwrap());
                        for _ in 0..4 {
                            if let Some(c) = chars.next() {
                                result.push(c);
                            }
                        }
                    }
                    _ => result.push_str("\\\\"),
                }
            }
            '"' => {
                in_string = !in_string;
                result.push('"');
            }
            '\n' if in_string => result.push_str("\\n"),
            '\r' if in_string => result.push_str("\\r"),
            '\t' if in_string => result.push_str("\\t"),
            ch if in_string && (ch as u32) < 0x20 => {
                result.push_str(&format!("\\u{:04x}", ch as u32));
            }
            _ => result.push(ch),
        }
    }

    result
}

fn remove_markdown_wrapper(content: &str) -> String {
    let trimmed = content.trim();

    let after_open = if let Some(rest) = trimmed.strip_prefix("```json") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("```") {
        rest
    } else {
        return trimmed.to_string();
    };

    if let Some(close_pos) = after_open.rfind("\n```") {
        return after_open[..close_pos].trim().to_string();
    }

    if let Some(inner) = after_open.trim_end().strip_suffix("```") {
        return inner.trim().to_string();
    }

    after_open.trim().to_string()
}
