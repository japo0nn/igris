use axum::{
    Router,
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tower_http::cors::{Any, CorsLayer};

use std::sync::{Arc, Mutex};

#[derive(Deserialize)]
pub struct ChatRequest {
    pub message: String,
}

#[derive(Serialize)]
pub struct ChatResponse {
    pub reply: String,
}

#[derive(Serialize)]
pub struct HistoryMessage {
    pub role: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct HistoryResponse {
    pub messages: Vec<HistoryMessage>,
}

#[derive(Clone)]
pub struct AppState {
    pub connection: Arc<Mutex<rusqlite::Connection>>,
    pub binary_path: String,
}

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/api/chat", post(chat_handler))
        .route("/api/history", get(history_handler))
        .route("/api/health", get(health_handler))
        .with_state(state)
        .layer(cors)
}

async fn health_handler() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "name": "IGRIS" }))
}

async fn history_handler(
    State(state): State<AppState>,
) -> Result<Json<HistoryResponse>, StatusCode> {
    let raw_messages = {
        let connection = state.connection.lock().unwrap();
        let mut stmt = connection.prepare(
            "SELECT role, content, is_done FROM messages WHERE role != 'system' ORDER BY timestamp ASC"
        ).unwrap();
        stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, bool>(2)?,
            ))
        })
        .unwrap()
        .filter_map(|r| r.ok())
        .collect::<Vec<_>>()
    };

    let history: Vec<HistoryMessage> = raw_messages
        .into_iter()
        .filter_map(|(role, content, is_done)| {
            // Parse JSON content to extract message field
            let text = if let Ok(v) = serde_json::from_str::<serde_json::Value>(&content) {
                v["message"].as_str().unwrap_or("").to_string()
            } else {
                content.clone()
            };

            // Skip system execution results (intermediate steps)
            if text.starts_with("[SYSTEM EXECUTION RESULT]") {
                return None;
            }
            // Skip empty messages
            if text.trim().is_empty() {
                return None;
            }
            // For assistant: only show final messages (is_done = true)
            if role == "assistant" && !is_done {
                return None;
            }

            Some(HistoryMessage {
                role,
                content: text,
            })
        })
        .collect();

    Ok(Json(HistoryResponse { messages: history }))
}

async fn chat_handler(
    State(state): State<AppState>,
    Json(payload): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, StatusCode> {
    let output = tokio::process::Command::new(&state.binary_path)
        .arg("--message")
        .arg(&payload.message)
        .output()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let raw = String::from_utf8_lossy(&output.stdout).to_string();

    // Parse JSON reply from output lines
    let reply = raw
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter_map(|v| v["message"].as_str().map(|s| s.to_string()))
        .last()
        .unwrap_or_else(|| raw.trim().to_string());

    Ok(Json(ChatResponse { reply }))
}
