use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};

use crate::{configs::llm::AppConfig, error::IgrisError, models::assistant::AssistantMessage};

pub async fn ask_llm(
    messages: &Vec<AssistantMessage>,
    config: &AppConfig,
    max_tokens: u32,
) -> Result<String, IgrisError> {
    let client = reqwest::Client::new();

    let request_body = serde_json::json!({
        "model": &config.llm.model,
        "messages": messages,
        "stream": false,
        "max_tokens": max_tokens
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

    Ok(remove_markdown_wrapper(&content))
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
