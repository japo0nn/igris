use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct SystemInfo {
    pub os: String,
    pub shell: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TopicRequest {
    pub message: String,
    pub existing_topics: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TaskObject {
    pub message: String,
    pub system_info: SystemInfo,
    pub system_response: Option<String>,
    pub skills: Vec<TaskObjectSkill>,
    pub all_topics: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TaskObjectSkill {
    pub name: String,
    pub description: String,
    pub available_methods: Vec<TaskObjectSkillMethod>,
}

#[derive(Debug, Deserialize, Serialize)]
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ActionResponse {
    pub message: String,
    pub is_done: bool,
    pub actions: Vec<Action>,
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
pub struct AssistanResponse {
    pub choices: Vec<AssistantChoice>,
}
