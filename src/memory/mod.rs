use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Session {
    pub id: Uuid,
    pub timestamp: DateTime<Local>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Message {
    pub id: Uuid,
    pub session_id: Uuid,
    pub role: String,
    pub content: String,
    pub action: Option<String>,
    pub is_done: bool,
    pub timestamp: DateTime<Local>,
}

pub struct MessageTopic {
    pub id: Uuid,
    pub message_id: Uuid,
    pub topic: String,
    pub timestamp: DateTime<Local>,
}
