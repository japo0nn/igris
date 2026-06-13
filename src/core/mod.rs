use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::configs::llm::AppConfig;

pub mod agent;
pub mod chat;
pub mod llm;
pub mod task;

#[derive(Debug, Clone)]
pub struct CoreContext {
    pub connection: Arc<Mutex<Connection>>,
    pub config: AppConfig,
}
