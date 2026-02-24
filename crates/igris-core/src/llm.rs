use crate::config::{LlmConfig, LlmMode};
use igris_memory::models::ChatMessage;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Deserialize, Debug)]
struct OllamaMessage {
    content: String,
}

#[derive(Deserialize, Debug)]
struct OllamaResponse {
    message: OllamaMessage,
}

#[derive(Deserialize, Debug)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

#[derive(Deserialize, Debug)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
}

#[derive(Deserialize, Debug)]
pub struct LlmAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub title: Option<String>,
    pub content: Option<String>,
    pub path: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct LlmOutput {
    pub message: String,
    pub actions: Vec<LlmAction>
}

pub fn send_message(config: &LlmConfig, history: &[ChatMessage]) -> Result<LlmOutput, String> {
    let client = Client::new();

    match config.mode {
        LlmMode::Offline => send_ollama(&client, config, history),
        LlmMode::Online => send_anthropic(&client, config, history),
    }
}

fn send_ollama(
    client: &Client,
    config: &LlmConfig,
    history: &[ChatMessage],
) -> Result<LlmOutput, String> {

    let mut messages: Vec<Message> = vec![
        Message{
            role: "system".to_string(),
            content: crate::prompt::SYSTEM_PROMPT.to_string()
        }
    ];

    messages.extend(history.iter().map(|m| Message { role: m.role.clone(), content: m.content.clone() }));

    let request_body = OllamaRequest {
        model: config.model.clone(),
        stream: false,
        messages: messages,
    };

    let response = client
        .post(&config.base_url)
        .json(&request_body)
        .send()
        .map_err(|e| format!("Ошибка запроса к Ollama: {}", e))?;

    let parsed: OllamaResponse = response
        .json()
        .map_err(|e| format!("Ошибка парсинга ответа Ollama: {}", e))?;

    let raw = parsed.message.content;
    let raw = strip_markdown_json(&raw);
    let output: LlmOutput = serde_json::from_str(&raw)
        .map_err(|e| format!("Ошибка парсинга JSON от LLM: {}\nСырой ответ: {}", e, raw))?;

    Ok(output)
}

fn send_anthropic(
    client: &Client,
    config: &LlmConfig,
    history: &[ChatMessage],
) -> Result<LlmOutput, String> {
    let api_key = config
        .api_key
        .as_ref()
        .ok_or("API ключ не найден в конфиге")?;

    let messages: Vec<Message> = history
        .iter()
        .map(|m| Message {
            role: m.role.clone(),
            content: m.content.clone(),
        })
        .collect();

    let request_body = AnthropicRequest {
        model: config.model.clone(),
        max_tokens: 1024,
        messages: messages,
    };

    let response = client
        .post(&config.base_url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&request_body)
        .send()
        .map_err(|e| format!("Ошибка запроса: {}", e))?;

    let parsed: AnthropicResponse = response
        .json()
        .map_err(|e| format!("Ошибка парсинга! Ответ: {}", e))?;

    let raw = parsed
        .content
        .into_iter()
        .find(|b| b.block_type == "text")
        .and_then(|b| b.text)
        .ok_or("Пустой ответ от LLM".to_string())?;

    let raw = strip_markdown_json(&raw);

    let output: LlmOutput = serde_json::from_str(&raw)
        .map_err(|e| format!("Ошибка парсинга JSON от LLM: {}\nСырой ответ: {}", e, raw))?;

    Ok(output)
}

fn strip_markdown_json(raw: &str) -> &str {
    let raw = raw.trim();
    let raw = raw.strip_prefix("```json").unwrap_or(raw);
    let raw = raw.strip_prefix("```").unwrap_or(raw);
    let raw = raw.strip_suffix("```").unwrap_or(raw);
    raw.trim()
}