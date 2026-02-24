use serde::Deserialize;
use std::fs;

// pub делает все публичным. В Rust все приватное по умолчанию!
#[derive(Deserialize, Debug)]
pub enum LlmMode {
    Online,
    Offline,
}

#[derive(Deserialize, Debug)] // derive используется для десериализации с JSON/TOML моделей.
pub struct LlmConfig {
    pub mode: LlmMode,
    pub model: String,
    pub base_url: String,
    pub api_key: Option<String>, // Option<> указывает что данный тип Nullable
}

#[derive(Deserialize, Debug)]
pub struct IntegrationsConfig {
    pub obsidian_vault: String,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub llm: LlmConfig,
    pub integrations: IntegrationsConfig,
}

pub fn load_config(path: &str) -> Result<Config, String> {
    let contents =
        fs::read_to_string(path).map_err(|e| format!("Не удалось прочитать файл: {}", e))?;

    let config: Config =
        toml::from_str(&contents).map_err(|e| format!("Ошибка парсинга конфига: {}", e))?;

    Ok(config)
}
