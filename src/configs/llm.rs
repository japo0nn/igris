use dirs;
use serde::Deserialize;

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub llm: LlmConfig,
    pub topic_llm: TopicLlmConfig,
    pub memory: MemoryConfig,
    pub execution: ExecutionConfig,
    pub telegram: Option<TelegramSecrets>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SecretsConfig {
    pub llm: LlmSecrets,
    pub voice: Option<VoiceSecrets>,
    pub telegram: Option<TelegramSecrets>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct VoiceSecrets {
    pub groq_api_key: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TelegramSecrets {
    pub api_id: i32,
    pub api_hash: String,
    pub phone_number: String,
    #[serde(default = "default_tg_session_path")]
    pub session_path: String,
}

impl TelegramSecrets {
    /// Basic validation: ensures required MTProto credentials are present and sane.
    pub fn is_valid(&self) -> bool {
        self.api_id > 0
            && !self.api_hash.trim().is_empty()
            && self.phone_number.trim().starts_with('+')
    }
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
pub struct ExecutionConfig {
    #[serde(default = "default_iteration_limit")]
    pub iteration_limit: u32,
    #[serde(default = "default_fix_iteration_limit")]
    pub fix_iteration_limit: u32,
}

fn default_iteration_limit() -> u32 {
    10
}
fn default_fix_iteration_limit() -> u32 {
    5
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
    #[serde(default = "default_retry_max_retries")]
    pub retry_max_retries: u32,
    #[serde(default = "default_retry_initial_delay_ms")]
    pub retry_initial_delay_ms: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct TopicLlmConfig {
    pub model: String,
    #[serde(default = "default_vision_model")]
    pub vision_model: String,
    pub system_prompt: String,
}

fn default_context_limit() -> usize {
    128000
}

fn default_retention_days() -> i32 {
    7
}

fn default_retry_max_retries() -> u32 {
    3
}

fn default_retry_initial_delay_ms() -> u64 {
    1000
}

fn default_tg_session_path() -> String {
    String::from("./igris_tg.session")
}

fn find_config(filename: &str) -> Result<String, Box<dyn std::error::Error>> {
    let paths = [
        format!("./{filename}"),
        format!(
            "{}/.config/igris/{}",
            dirs::home_dir().unwrap().display(),
            filename
        ),
    ];
    for p in &paths {
        if std::path::Path::new(p).exists() {
            return std::fs::read_to_string(p).map_err(|e| e.into());
        }
    }
    eprintln!("[IGRIS] Config file not found. Checked paths:");
    for p in &paths {
        eprintln!("  - {}", p);
    }
    Err(Box::new(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        format!("Config {} not found", filename),
    )))
}

pub fn load_config() -> Result<(AppConfig, SecretsConfig), Box<dyn std::error::Error>> {
    let content = find_config("config.toml")?;
    let mut config: AppConfig = toml::from_str(&content)?;

    let secrets_content = find_config("secrets.toml")?;
    let secrets: SecretsConfig = toml::from_str(&secrets_content)?;

    config.llm.api_key = Some(secrets.llm.api_key.clone());
    config.telegram = secrets.telegram.clone();

    Ok((config, secrets))
}

fn default_vision_model() -> String {
    String::from("cc/claude-sonnet-4-6")
}
