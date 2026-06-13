use crate::skills::SkillError;

#[derive(Debug)]
pub enum IgrisError {
    LlmUnavailable(String),
    ParseError(String),
    SkillNotFound(String),
    SkillError(String),
    DatabaseError(String),
    ConfigError(String),
    IoError(String),
}

impl std::fmt::Display for IgrisError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            IgrisError::LlmUnavailable(msg) => write!(f, "LLM unavailable: {}", msg),
            IgrisError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            IgrisError::SkillNotFound(msg) => write!(f, "Skill not found: {}", msg),
            IgrisError::SkillError(msg) => write!(f, "Skill error: {}", msg),
            IgrisError::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            IgrisError::ConfigError(msg) => write!(f, "Config error: {}", msg),
            IgrisError::IoError(msg) => write!(f, "IO error: {}", msg),
        }
    }
}

impl std::error::Error for IgrisError {}

impl From<serde_json::Error> for IgrisError {
    fn from(value: serde_json::Error) -> Self {
        IgrisError::ParseError(value.to_string())
    }
}

impl From<toml::de::Error> for IgrisError {
    fn from(value: toml::de::Error) -> Self {
        IgrisError::ParseError(value.to_string())
    }
}

impl From<reqwest::Error> for IgrisError {
    fn from(value: reqwest::Error) -> Self {
        IgrisError::LlmUnavailable(value.to_string())
    }
}

impl From<std::io::Error> for IgrisError {
    fn from(value: std::io::Error) -> Self {
        IgrisError::IoError(value.to_string())
    }
}

impl From<rusqlite::Error> for IgrisError {
    fn from(value: rusqlite::Error) -> Self {
        IgrisError::DatabaseError(value.to_string())
    }
}

impl From<SkillError> for IgrisError {
    fn from(value: SkillError) -> Self {
        match value {
            SkillError::NotFound(msg) => IgrisError::SkillNotFound(msg),
            SkillError::ExecutionFailed(msg) => IgrisError::SkillError(msg),
            SkillError::InvalidArgs(msg) => IgrisError::SkillError(msg),
        }
    }
}
