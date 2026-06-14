use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};

use crate::{configs::llm::AppConfig, error::IgrisError, models::assistant::AssistantMessage};

pub async fn ask_llm(
    messages: &Vec<AssistantMessage>,
    config: &AppConfig,
    _max_tokens: u32,
) -> Result<String, IgrisError> {
    let client = reqwest::Client::new();

    let request_body = serde_json::json!({
        "model": &config.llm.model,
        "messages": messages,
        "stream": false,
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

    println!("API RESPONSE: {}", response);

    return Ok(extract_content(&response)?);
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
        .to_string()
        .replace("", "")
        .replace("", "");

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
                    // Валидные JSON escapes — копируем оба символа как есть
                    Some(&'"') | Some(&'\\') | Some(&'/') | Some(&'b') | Some(&'f')
                    | Some(&'n') | Some(&'r') | Some(&'t') => {
                        result.push('\\');
                        result.push(chars.next().unwrap());
                    }
                    // \uXXXX — копируем все 6 символов
                    Some(&'u') => {
                        result.push('\\');
                        result.push(chars.next().unwrap()); // 'u'
                        for _ in 0..4 {
                            if let Some(c) = chars.next() {
                                result.push(c);
                            }
                        }
                    }
                    // \x, \e, \1, и любые другие невалидные — экранируем бэкслэш
                    _ => result.push_str("\\\\"),
                }
            }
            '"' => {
                in_string = !in_string;
                result.push('"');
            }
            // Реальные control chars внутри строки — экранируем
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
