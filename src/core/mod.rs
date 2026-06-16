use std::sync::{Arc, Mutex};

use rusqlite::Connection;

use crate::configs::llm::AppConfig;
use crate::supervisor::Supervisor;

pub mod agent;
pub mod chat;
pub mod llm;
pub mod markdown;
pub mod spinner;
pub mod task;
pub mod utils;

#[derive(Debug, Clone)]
pub struct CoreContext {
    pub connection: Arc<Mutex<Connection>>,
    pub config: AppConfig,
    pub spinner: crate::core::spinner::Spinner,
    pub supervisor: Supervisor,
}
