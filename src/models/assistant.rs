use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SystemInfo {
    pub os: String,
    pub shell: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TopicRequest {
    pub message: String,
    pub existing_topics: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Constraints {
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
    #[serde(default = "default_max_fix_iterations")]
    pub max_fix_iterations: u32,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
}

fn default_max_iterations() -> u32 { 10 }
fn default_max_fix_iterations() -> u32 { 5 }
fn default_max_tokens() -> u32 { 16000 }

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TaskObject {
    pub message: String,
    pub system_info: SystemInfo,
    pub system_response: Option<String>,
    pub skills: Vec<TaskObjectSkill>,
    pub all_topics: Vec<String>,
    pub capabilities: Vec<String>,
    pub constraints: Constraints,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TaskObjectSkill {
    pub name: String,
    pub description: String,
    pub available_methods: Vec<TaskObjectSkillMethod>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct TaskObjectSkillMethod {
    pub method: String,
    pub description: String,
    pub args_description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum Action {
    ExecuteModule {
        module: String,
        method: String,
        args: String,
    },
    RespondToUser,
    PermissionRequest {
        action: String,
        description: String,
        risk_level: String,
        options: Vec<String>,
    },
    PromptUser {
        message: String,
        options: Vec<String>,
    },
    RequestData {
        source: String,
        query: String,
        limit: u32,
    },
    GenerateChunk {
        module_name: String,
        chunk_index: u32,
        total_chunks: u32,
        code_chunk: String,
        dependencies: Vec<String>,
    },
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActionResponse {
    pub message: String,
    pub is_done: bool,
    pub actions: Vec<Action>,
    #[serde(default)]
    pub iteration: u32,
    #[serde(default)]
    pub fix_iteration: u32,
    #[serde(default)]
    pub constraints: Option<Constraints>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AssistantMessage {
    pub role: String,
    pub content: String,
}

#[derive(Deserialize, Debug)]
pub struct AssistantChoice {
    pub message: AssistantMessage,
}

#[derive(Deserialize, Debug)]
pub struct AssistantResponse {
    pub choices: Vec<AssistantChoice>,
}
