use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};

use crate::{configs::llm::AppConfig, error::IgrisError, models::assistant::AssistantMessage};

pub async fn ask_llm(
    messages: &Vec<AssistantMessage>,
    config: &AppConfig,
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
            "thinking": {
                "type": "disabled"
            }
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
            Ok(response) => match response.text().await {
                Ok(text) => {
                    // println!("{}", text);
                    return Ok(extract_content(&text)?);
                }
                Err(e) => {
                    last_error = Some(IgrisError::LlmUnavailable(format!(
                        "Response read failed: {}",
                        e
                    )));
                }
            },
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

    Err(last_error.unwrap_or(IgrisError::LlmUnavailable(
        "All retries exhausted".to_string(),
    )))
}

pub async fn generate_topics(message: String, config: &AppConfig) -> Result<String, IgrisError> {
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
        .unwrap_or_default();

    Ok(extract_json_payload(content))
}

fn extract_json_payload(content: &str) -> String {
    let trimmed = content.trim();

    if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
        return trimmed.to_string();
    }

    let candidate = match extract_braced_substring(trimmed) {
        Some(c) => c,
        None => return trimmed.to_string(),
    };

    if serde_json::from_str::<serde_json::Value>(&candidate).is_ok() {
        return candidate;
    }

    escape_raw_control_chars_in_strings(&candidate)
}

fn extract_braced_substring(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let start = chars.iter().position(|&c| c == '{' || c == '[')?;

    let open = chars[start];
    let close = if open == '{' { '}' } else { ']' };

    let mut depth = 0i32;
    let mut in_string = false;
    let mut escaped = false;
    let mut end: Option<usize> = None;

    for i in start..chars.len() {
        let c = chars[i];

        if escaped {
            escaped = false;
            continue;
        }

        match c {
            '\\' if in_string => escaped = true,
            '"' => in_string = !in_string,
            c if !in_string && c == open => depth += 1,
            c if !in_string && c == close => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }

    end.map(|e| chars[start..=e].iter().collect())
}

fn escape_raw_control_chars_in_strings(json: &str) -> String {
    let mut result = String::with_capacity(json.len() + 16);
    let mut in_string = false;
    let mut escaped = false;

    for ch in json.chars() {
        if escaped {
            result.push(ch);
            escaped = false;
            continue;
        }

        match ch {
            '\\' if in_string => {
                escaped = true;
                result.push(ch);
            }
            '"' => {
                in_string = !in_string;
                result.push(ch);
            }
            '\n' if in_string => result.push_str("\\n"),
            '\r' if in_string => result.push_str("\\r"),
            '\t' if in_string => result.push_str("\\t"),
            c if in_string && (c as u32) < 0x20 => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            _ => result.push(ch),
        }
    }

    result
}
