use crate::skills::SkillError;
use rustyline::error::ReadlineError;
use std::fmt;

/// Основные типы ошибок IGRIS
#[derive(Debug, Clone)]
pub enum IgrisError {
    // LLM-связанные ошибки
    LlmUnavailable(String),
    LlmTimeout(String),
    LlmInvalidResponse(String),

    // Validator ошибки
    ValidatorRejected(String, usize),
    ValidatorTestsFailed(String),

    // Итерационные ошибки
    MaxIterationsExceeded(usize),
    MaxFixIterationsExceeded(usize),

    // Sandbox ошибки
    SandboxExecutionFailed(String),
    SandboxTimeout,
    SandboxResourceLimitExceeded,

    // Chunk/Module ошибки
    InvalidChunkSyntax(String, usize),
    ModuleCompilationFailed(String),

    // Parallel execution ошибки
    SubtaskFailed(String, String),
    ParallelExecutionAborted(Vec<String>),

    // Базовые ошибки
    ParseError(String),
    SkillNotFound(String),
    SkillError(String),
    DatabaseError(String),
    ConfigError(String),
    IoError(String),
    PermissionDenied(String),
    ReadlineError(String),
}

impl fmt::Display for IgrisError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            IgrisError::LlmUnavailable(msg) => write!(f, "[LLM ERROR] Unavailable: {}", msg),
            IgrisError::LlmTimeout(msg) => write!(f, "[LLM ERROR] Timeout: {}", msg),
            IgrisError::LlmInvalidResponse(msg) => write!(
                f,
                "[LLM ERROR] Invalid response format of IGRIS(should to remember message format from system prompt): {}",
                msg
            ),
            IgrisError::ValidatorRejected(msg, iter) => write!(
                f,
                "[VALIDATOR ERROR] Rejected (iteration {}): {}",
                iter, msg
            ),
            IgrisError::ValidatorTestsFailed(msg) => {
                write!(f, "[VALIDATOR ERROR] Tests failed: {}", msg)
            }
            IgrisError::MaxIterationsExceeded(max) => {
                write!(f, "[LOOP ERROR] Max iterations exceeded: {}", max)
            }
            IgrisError::MaxFixIterationsExceeded(max) => {
                write!(f, "[FIX ERROR] Max fix iterations exceeded: {}", max)
            }
            IgrisError::SandboxExecutionFailed(msg) => {
                write!(f, "[SANDBOX ERROR] Execution failed: {}", msg)
            }
            IgrisError::SandboxTimeout => write!(f, "[SANDBOX ERROR] Timeout"),
            IgrisError::SandboxResourceLimitExceeded => {
                write!(f, "[SANDBOX ERROR] Resource limit exceeded")
            }
            IgrisError::InvalidChunkSyntax(code, idx) => {
                write!(f, "[CHUNK ERROR] Syntax error in chunk {}: {}", idx, code)
            }
            IgrisError::ModuleCompilationFailed(msg) => {
                write!(f, "[MODULE ERROR] Compilation failed: {}", msg)
            }
            IgrisError::SubtaskFailed(id, msg) => {
                write!(f, "[PARALLEL ERROR] Subtask {} failed: {}", id, msg)
            }
            IgrisError::ParallelExecutionAborted(ids) => write!(
                f,
                "[PARALLEL ERROR] Execution aborted. Failed tasks: {:?}",
                ids
            ),
            IgrisError::ParseError(msg) => write!(f, "[PARSE ERROR] {}", msg),
            IgrisError::SkillNotFound(msg) => write!(f, "[SKILL ERROR] Not found: {}", msg),
            IgrisError::SkillError(msg) => write!(f, "[SKILL ERROR] {}", msg),
            IgrisError::DatabaseError(msg) => write!(f, "[DB ERROR] {}", msg),
            IgrisError::ConfigError(msg) => write!(f, "[CONFIG ERROR] {}", msg),
            IgrisError::IoError(msg) => write!(f, "[IO ERROR] {}", msg),
            IgrisError::PermissionDenied(msg) => write!(f, "[PERMISSION ERROR] {}", msg),
            IgrisError::ReadlineError(msg) => write!(f, "[READLINE ERROR] {}", msg),
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
        if value.is_timeout() {
            IgrisError::LlmTimeout(value.to_string())
        } else {
            IgrisError::LlmUnavailable(value.to_string())
        }
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

impl From<ReadlineError> for IgrisError {
    fn from(value: ReadlineError) -> Self {
        IgrisError::ReadlineError(value.to_string())
    }
}

pub type IgrisResult<T> = Result<T, IgrisError>;
