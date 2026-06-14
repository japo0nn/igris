use serde_json::Value;

use crate::{error::IgrisError, models::metadata::ModuleMetadata};

pub mod memory_skill;
pub mod gui_skill;
pub mod shell_executor;
pub mod user_profile_skill;
pub mod web_search_skill;

pub trait SkillModule {
    fn get_metadata(&self) -> &ModuleMetadata;
    fn health_check(&self) -> bool;
    fn execute(&self, method: &str, args: &str) -> Result<SkillOutput, SkillError>;
    fn available_methods(&self) -> Vec<MethodInfo>;
}

pub fn find_skill<'a>(
    skills: &'a Vec<Box<dyn SkillModule>>,
    name: &str,
) -> Result<&'a Box<dyn SkillModule>, IgrisError> {
    let result = skills
        .iter()
        .find(|x| x.get_metadata().name.to_lowercase() == name.to_lowercase());

    match result {
        Some(skill) => Ok(skill),
        None => Err(IgrisError::SkillNotFound(format!(
            "Skill not found: {}",
            name
        ))),
    }
}

pub struct MethodInfo {
    pub method: String,
    pub description: String,
    pub args_description: String,
}

pub enum SkillOutput {
    Text(String),
    Json(Value),
    Binary(Vec<u8>),
    Empty,
}

pub enum SkillError {
    NotFound(String),
    ExecutionFailed(String),
    InvalidArgs(String),
}

impl std::fmt::Display for SkillError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            SkillError::NotFound(msg) => write!(f, "Skill not found: {}", msg),
            SkillError::ExecutionFailed(msg) => write!(f, "Execution failed: {}", msg),
            SkillError::InvalidArgs(msg) => write!(f, "Invalid args: {}", msg),
        }
    }
}

impl From<std::io::Error> for SkillError {
    fn from(value: std::io::Error) -> Self {
        SkillError::ExecutionFailed(value.to_string())
    }
}

impl From<rusqlite::Error> for SkillError {
    fn from(value: rusqlite::Error) -> Self {
        SkillError::ExecutionFailed(value.to_string())
    }
}
