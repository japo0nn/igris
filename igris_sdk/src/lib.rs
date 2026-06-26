use serde::{Deserialize, Serialize};
use std::fmt;

/// Module metadata - required for every skill
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleMetadata {
    pub name: String,
    pub version: String,
    pub description: String,
}

/// A method that a skill exposes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodInfo {
    pub method: String,
    pub description: String,
    pub args_description: String,
}

/// Output types from skill execution
pub enum SkillOutput {
    Text(String),
    Json(serde_json::Value),
    Binary(Vec<u8>),
    Empty,
}

/// Errors that can occur during skill execution
#[derive(Debug)]
pub enum SkillError {
    NotFound(String),
    ExecutionFailed(String),
    InvalidArgs(String),
    Recoverable(String),
}

impl fmt::Display for SkillError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SkillError::NotFound(msg) => write!(f, "Skill not found: {}", msg),
            SkillError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            SkillError::InvalidArgs(msg) => write!(f, "Invalid args: {}", msg),
            SkillError::Recoverable(msg) => write!(f, "Recoverable error: {}", msg),
        }
    }
}

impl std::error::Error for SkillError {}

impl From<std::io::Error> for SkillError {
    fn from(value: std::io::Error) -> Self {
        SkillError::ExecutionFailed(value.to_string())
    }
}

/// The main trait that every skill must implement
#[async_trait::async_trait]
pub trait SkillModule: Send + Sync {
    fn get_metadata(&self) -> &ModuleMetadata;
    fn health_check(&self) -> bool;
    async fn execute(&self, method: &str, args: &str) -> Result<SkillOutput, SkillError>;
    fn available_methods(&self) -> Vec<MethodInfo>;
}
