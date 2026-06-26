use async_trait::async_trait;
use chrono::{DateTime, FixedOffset, Local, NaiveDateTime};
use rusqlite::params;
use uuid::Uuid;

use crate::{
    core::CoreContext,
    memory::Message,
    models::metadata::ModuleMetadata,
    skills::{MethodInfo, SkillError, SkillModule, SkillOutput},
};

#[derive(Debug, Clone)]
pub struct MemorySkill {
    pub metadata: ModuleMetadata,
    pub context: CoreContext,
}

impl MemorySkill {
    pub fn new(context: CoreContext) -> Self {
        MemorySkill {
            metadata: ModuleMetadata {
                name: "Memory".to_string(),
                version: "0.1.0".to_string(),
                _type: crate::models::metadata::ModuleType::Persistent,
                description: "Memory Skill is connected to IGRIS database where stored user\'s and assistant all messages".to_string(),
                author: Some("IGRIS".to_string()),
            },
            context,
        }
    }
}

#[async_trait]
impl SkillModule for MemorySkill {
    fn get_metadata(&self) -> &ModuleMetadata {
        &self.metadata
    }

    fn health_check(&self) -> bool {
        true
    }

    async fn execute(&self, method: &str, args: &str) -> Result<SkillOutput, SkillError> {
        let connection = &self
            .context
            .connection
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        match method {
            "by-topics" => self.by_topics(args, connection),
            "get-sessions" => self.get_sessions(connection),
            "get-messages-by-time-range" => self.by_time_range(args, connection),
            "get-messages-paginated" => self.paginated(args, connection),
            "get-messages-by-session" => self.by_session(args, connection),
            "get-topics" => self.get_topics(connection),
            "search-messages" => self.search_messages(args, connection),
            "get-message-by-id" => self.by_message_id(args, connection),
            "get-sessions-by-date" => self.sessions_by_date(args, connection),
            _ => Err(SkillError::InvalidArgs(format!(
                "Method '{}' does not exist",
                method
            ))),
        }
    }

    fn available_methods(&self) -> Vec<MethodInfo> {
        vec![
            MethodInfo {
                method: String::from("by-topics"),
                description: String::from(
                    "CRITICAL: Use this to retrieve the main context of a specific subject. Better than pulling the whole session history.",
                ),
                args_description: String::from(
                    "Space-separated list of topic names. Example: birthday greeting",
                ),
            },
            MethodInfo {
                method: String::from("get-sessions"),
                description: String::from(
                    "Use this to identify which session contains the needed information before fetching messages.",
                ),
                args_description: String::from(
                    "No arguments required. Pass an empty string or a single space.",
                ),
            },
            MethodInfo {
                method: String::from("get-messages-by-time-range"),
                description: String::from(
                    "Use this to isolate events within a specific timeframe.",
                ),
                args_description: String::from(
                    "Start and end timestamps separated by a pipe character '|'. Example: 2026-06-13 06:10:27|2026-06-14 06:10:27",
                ),
            },
            MethodInfo {
                method: String::from("get-messages-paginated"),
                description: String::from(
                    "Use this to explore long conversations without overloading the token context window.",
                ),
                args_description: String::from(
                    "Page number and page size separated by a space. Example: 1 10",
                ),
            },
            MethodInfo {
                method: String::from("get-messages-by-session"),
                description: String::from(
                    "Use ONLY when a precise chronological sequence of events in a specific session is required.",
                ),
                args_description: String::from(
                    "Session UUID. Example: 4ff03094-df9e-4f14-94a8-38b69e19ed36",
                ),
            },
            MethodInfo {
                method: String::from("get-topics"),
                description: String::from(
                    "Use this to discover what subjects have been discussed across all time.",
                ),
                args_description: String::from(
                    "No arguments required. Pass an empty string or a single space.",
                ),
            },
            MethodInfo {
                method: String::from("search-messages"),
                description: String::from(
                    "HIGH PRIORITY: Use for searching specific code snippets, function names, or unique terms across all history.",
                ),
                args_description: String::from("A keyword or phrase to search for. Example: hello"),
            },
            MethodInfo {
                method: String::from("get-message-by-id"),
                description: String::from("Fetch a specific message by ID for absolute precision."),
                args_description: String::from(
                    "Message UUID. Example: 10bb4ee9-a3fc-4034-89b2-757b558531ba",
                ),
            },
            MethodInfo {
                method: String::from("get-sessions-by-date"),
                description: String::from(
                    "Find sessions from a specific date range to narrow down the search area.",
                ),
                args_description: String::from(
                    "Start and end timestamps separated by a pipe character '|'. Example: 2026-06-13 06:00:00|2026-06-14 06:00:00",
                ),
            },
        ]
    }
}

impl MemorySkill {
    fn by_topics(
        &self,
        args: &str,
        connection: &rusqlite::Connection,
    ) -> Result<SkillOutput, SkillError> {
        let topics: Vec<&str> = args.split_whitespace().filter(|s| !s.is_empty()).collect();
        if topics.is_empty() {
            return Ok(SkillOutput::Text("[]".to_string()));
        }

        let placeholders: Vec<String> = topics
            .iter()
            .enumerate()
            .map(|(i, _)| format!("?{}", i + 1))
            .collect();
        let placeholders_str = placeholders.join(", ");

        let query = format!(
            "SELECT DISTINCT m.id, m.session_id, m.role, m.content, m.timestamp, m.action, m.is_done
             FROM messages m
             JOIN message_topics t ON m.id = t.message_id
             WHERE t.topic IN ({})
             ORDER BY m.timestamp DESC",
            placeholders_str
        );

        let mut stmt = connection.prepare(&query)?;
        let params: Vec<&dyn rusqlite::ToSql> =
            topics.iter().map(|t| &*t as &dyn rusqlite::ToSql).collect();

        let messages = stmt
            .query_map(params.as_slice(), map_row)?
            .collect::<Result<Vec<_>, _>>()?;

        let json = serde_json::json!(messages).to_string();
        Ok(SkillOutput::Text(json))
    }

    fn get_sessions(&self, connection: &rusqlite::Connection) -> Result<SkillOutput, SkillError> {
        let mut stmt = connection
            .prepare("SELECT DISTINCT id, timestamp FROM sessions ORDER BY timestamp DESC")?;

        let sessions = stmt
            .query_map([], |row| {
                let id_str: String = row.get(0)?;
                let ts_str: String = row.get(1)?;
                Ok(serde_json::json!({
                    "id": id_str,
                    "timestamp": ts_str
                }))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let json = serde_json::json!(sessions).to_string();
        Ok(SkillOutput::Text(json))
    }

    fn by_time_range(
        &self,
        args: &str,
        connection: &rusqlite::Connection,
    ) -> Result<SkillOutput, SkillError> {
        let parts: Vec<&str> = args.splitn(2, '|').collect();
        if parts.len() != 2 {
            return Err(SkillError::InvalidArgs("Usage: <start_timestamp>|<end_timestamp>. Example: 2026-06-13 06:10:27|2026-06-14 06:10:27".to_string()));
        }

        let start_str = parts[0].trim();
        let end_str = parts[1].trim();

        NaiveDateTime::parse_from_str(start_str, "%Y-%m-%d %H:%M:%S").map_err(|e| {
            SkillError::InvalidArgs(format!("Invalid start timestamp '{}': {}", start_str, e))
        })?;
        NaiveDateTime::parse_from_str(end_str, "%Y-%m-%d %H:%M:%S").map_err(|e| {
            SkillError::InvalidArgs(format!("Invalid end timestamp '{}': {}", end_str, e))
        })?;

        let mut stmt = connection.prepare(
            "SELECT id, session_id, role, content, timestamp, action, is_done FROM messages
             WHERE timestamp >= ?1 AND timestamp <= ?2
             ORDER BY timestamp DESC",
        )?;

        let messages = stmt
            .query_map(params![start_str, end_str], map_row)?
            .collect::<Result<Vec<_>, _>>()?;

        let json = serde_json::json!(messages).to_string();
        Ok(SkillOutput::Text(json))
    }

    fn paginated(
        &self,
        args: &str,
        connection: &rusqlite::Connection,
    ) -> Result<SkillOutput, SkillError> {
        let parts: Vec<&str> = args.split_whitespace().collect();
        if parts.len() != 2 {
            return Err(SkillError::InvalidArgs(
                "Usage: <page_number> <page_size>. Example: 1 10".to_string(),
            ));
        }

        let page: u32 = parts[0]
            .parse()
            .map_err(|_| SkillError::InvalidArgs("Page must be a positive integer".to_string()))?;
        let page_size: u32 = parts[1].parse().map_err(|_| {
            SkillError::InvalidArgs("Page size must be a positive integer".to_string())
        })?;

        if page == 0 {
            return Err(SkillError::InvalidArgs(
                "Page must be a positive integer".to_string(),
            ));
        }
        if page_size == 0 || page_size > 100 {
            return Err(SkillError::InvalidArgs(
                "Page size must be between 1 and 100".to_string(),
            ));
        }

        let offset = (page - 1) * page_size;

        let mut stmt = connection.prepare(
            "SELECT id, session_id, role, content, timestamp, action, is_done FROM messages
             ORDER BY timestamp DESC
             LIMIT ?1 OFFSET ?2",
        )?;

        let messages = stmt
            .query_map(params![page_size, offset], map_row)?
            .collect::<Result<Vec<_>, _>>()?;

        let json = serde_json::json!(messages).to_string();
        Ok(SkillOutput::Text(json))
    }

    fn by_session(
        &self,
        args: &str,
        connection: &rusqlite::Connection,
    ) -> Result<SkillOutput, SkillError> {
        let session_id = args.trim();
        if session_id.is_empty() {
            return Err(SkillError::InvalidArgs(
                "Please provide a session UUID".to_string(),
            ));
        }

        let mut stmt = connection.prepare(
            "SELECT id, session_id, role, content, timestamp, action, is_done FROM messages
             WHERE session_id = ?1
             ORDER BY timestamp ASC",
        )?;

        let messages = stmt
            .query_map(params![session_id], map_row)?
            .collect::<Result<Vec<_>, _>>()?;

        let json = serde_json::json!(messages).to_string();
        Ok(SkillOutput::Text(json))
    }

    fn get_topics(&self, connection: &rusqlite::Connection) -> Result<SkillOutput, SkillError> {
        let mut stmt =
            connection.prepare("SELECT DISTINCT topic FROM message_topics ORDER BY topic ASC")?;

        let topics = stmt
            .query_map([], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;

        let json = serde_json::json!(topics).to_string();
        Ok(SkillOutput::Text(json))
    }

    fn search_messages(
        &self,
        args: &str,
        connection: &rusqlite::Connection,
    ) -> Result<SkillOutput, SkillError> {
        let keyword = args.trim();
        if keyword.is_empty() {
            return Err(SkillError::InvalidArgs(
                "Please provide a search keyword".to_string(),
            ));
        }

        let pattern = format!("%{}%", keyword);

        let mut stmt = connection.prepare(
            "SELECT id, session_id, role, content, timestamp, action, is_done FROM messages
             WHERE content LIKE ?1
             ORDER BY timestamp DESC
             LIMIT 50",
        )?;

        let messages = stmt
            .query_map(params![pattern], map_row)?
            .collect::<Result<Vec<_>, _>>()?;

        let json = serde_json::json!(messages).to_string();
        Ok(SkillOutput::Text(json))
    }

    fn by_message_id(
        &self,
        args: &str,
        connection: &rusqlite::Connection,
    ) -> Result<SkillOutput, SkillError> {
        let msg_id = args.trim();
        if msg_id.is_empty() {
            return Err(SkillError::InvalidArgs(
                "Please provide a message UUID".to_string(),
            ));
        }

        let mut stmt = connection.prepare(
            "SELECT id, session_id, role, content, timestamp, action, is_done FROM messages
             WHERE id = ?1",
        )?;

        let messages = stmt
            .query_map(params![msg_id], map_row)?
            .collect::<Result<Vec<_>, _>>()?;

        let json = serde_json::json!(messages).to_string();
        Ok(SkillOutput::Text(json))
    }

    fn sessions_by_date(
        &self,
        args: &str,
        connection: &rusqlite::Connection,
    ) -> Result<SkillOutput, SkillError> {
        let parts: Vec<&str> = args.splitn(2, '|').collect();
        if parts.len() != 2 {
            return Err(SkillError::InvalidArgs("Usage: <start_timestamp>|<end_timestamp>. Example: 2026-06-13 06:00:00|2026-06-14 06:00:00".to_string()));
        }

        let start_str = parts[0].trim();
        let end_str = parts[1].trim();

        NaiveDateTime::parse_from_str(start_str, "%Y-%m-%d %H:%M:%S").map_err(|e| {
            SkillError::InvalidArgs(format!("Invalid start timestamp '{}': {}", start_str, e))
        })?;
        NaiveDateTime::parse_from_str(end_str, "%Y-%m-%d %H:%M:%S").map_err(|e| {
            SkillError::InvalidArgs(format!("Invalid end timestamp '{}': {}", end_str, e))
        })?;

        let mut stmt = connection.prepare(
            "SELECT DISTINCT id, timestamp FROM sessions
             WHERE timestamp >= ?1 AND timestamp <= ?2
             ORDER BY timestamp DESC",
        )?;

        let sessions = stmt
            .query_map(params![start_str, end_str], |row| {
                let id_str: String = row.get(0)?;
                let ts_str: String = row.get(1)?;
                Ok(serde_json::json!({
                    "id": id_str,
                    "timestamp": ts_str
                }))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let json = serde_json::json!(sessions).to_string();
        Ok(SkillOutput::Text(json))
    }
}

fn map_row(row: &rusqlite::Row) -> rusqlite::Result<Message> {
    Ok(Message { raw_json: None,
        id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or_else(|_| Uuid::nil()),
        session_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or_else(|_| Uuid::nil()),
        role: row.get(2)?,
        content: row.get(3)?,
        timestamp: parse_timestamp(&row.get::<_, String>(4)?).unwrap_or_else(|_| Local::now()),
        action: row.get::<_, Option<String>>(5)?,
        is_done: row.get::<_, bool>(6)?,
    })
}

fn parse_timestamp(value: &str) -> Result<DateTime<Local>, SkillError> {
    if let Ok(dt) = DateTime::<FixedOffset>::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f %:z") {
        return Ok(dt.with_timezone(&Local));
    }
    if let Ok(dt) = DateTime::<FixedOffset>::parse_from_str(value, "%Y-%m-%d %H:%M:%S %:z") {
        return Ok(dt.with_timezone(&Local));
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
        return Ok(naive.and_local_timezone(Local).unwrap());
    }
    if let Ok(naive) = NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f") {
        return Ok(naive.and_local_timezone(Local).unwrap());
    }
    Err(SkillError::ExecutionFailed(format!(
        "Cannot parse timestamp: {}",
        value
    )))
}
