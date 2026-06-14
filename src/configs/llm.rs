use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub llm: LlmConfig,
    pub topic_llm: TopicLlmConfig,
    pub memory: MemoryConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SecretsConfig {
    pub llm: LlmSecrets,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LlmSecrets {
    pub api_key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MemoryConfig {
    pub db_path: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LlmConfig {
    #[serde(default)]
    pub api_key: Option<String>,
    pub model: String,
    #[serde(default = "default_vision_model")]
    pub vision_model: String,
    pub base_uri: String,
    pub system_prompt: String,
    #[serde(default = "default_context_limit")]
    pub context_token_limit: usize,
    #[serde(default = "default_retention_days")]
    pub retention_days: i32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TopicLlmConfig {
    pub model: String,
    #[serde(default = "default_vision_model")]
    pub vision_model: String,
    pub system_prompt: String,
    #[serde(default = "default_topic_max_tokens")]
    pub max_tokens: u32,
}

fn default_context_limit() -> usize {
    128000
}

fn default_retention_days() -> i32 {
    7
}

fn default_max_tokens() -> u32 {
    16000
}

fn default_topic_max_tokens() -> u32 {
    1024
}

pub fn load_config() -> Result<(AppConfig, SecretsConfig), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string("./config.toml")?;
    let mut config: AppConfig = toml::from_str(&content)?;

    let secrets_content = std::fs::read_to_string("./secrets.toml")?;
    let secrets: SecretsConfig = toml::from_str(&secrets_content)?;

    config.llm.api_key = Some(secrets.llm.api_key.clone());

    Ok((config, secrets))
}

fn default_vision_model() -> String { String::from("cc/claude-sonnet-4-6") }
