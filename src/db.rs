use chrono::Local;
use rusqlite::{Connection, params};
use uuid::Uuid;

use crate::{
    configs::llm::AppConfig,
    core::utils::parse_db_timestamp,
    error::IgrisError,
    memory::{Message, MessageTopic, Session},
    models::assistant::ActionResponse,
};

pub fn init_database(config: &AppConfig) -> Result<Connection, IgrisError> {
    let db_path = std::path::Path::new(&config.memory.db_path);
    if let Some(parent) = db_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let conn = Connection::open(db_path)?;
    conn.execute("PRAGMA foreign_keys = ON;", [])?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, timestamp TEXT)",
        (),
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY,
            session_id TEXT,
            role TEXT,
            content TEXT,
            raw_json TEXT DEFAULT NULL,
            action TEXT,
            is_done INTEGER,
            timestamp TEXT,
            FOREIGN KEY(session_id) REFERENCES sessions(id) ON DELETE CASCADE
        )",
        (),
    )?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS message_topics (
            id TEXT PRIMARY KEY,
            message_id TEXT,
            topic TEXT,
            timestamp TEXT,
            FOREIGN KEY(message_id) REFERENCES messages(id) ON DELETE CASCADE
        )",
        (),
    )?;
    Ok(conn)
}

pub fn create_session(connection: &Connection) -> Result<Session, IgrisError> {
    let session = Session {
        id: Uuid::new_v4(),
        timestamp: Local::now(),
    };
    connection.execute(
        "INSERT INTO sessions (id, timestamp) VALUES (?1, ?2)",
        (&session.id.to_string(), &session.timestamp.to_string()),
    )?;
    return Ok(session);
}

pub fn insert_message(
    connection: &Connection,
    role: String,
    message: &ActionResponse,
    raw_json: Option<&str>,
    session: &Session,
) -> Result<Uuid, IgrisError> {
    let msg = Message {
        id: Uuid::new_v4(),
        session_id: session.id,
        role: role,
        content: message.message.clone(),
        raw_json: raw_json.map(|s| s.to_string()),
        action: Some(serde_json::json!(&message.actions).to_string()),
        is_done: message.is_done,
        timestamp: Local::now(),
    };
    connection.execute(
        "INSERT INTO messages (id, timestamp, session_id, role, content, raw_json, action, is_done) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        (
            &msg.id.to_string(),
            &msg.timestamp.to_string(),
            &msg.session_id.to_string(),
            &msg.role,
            &msg.content,
            &msg.raw_json,
            &msg.action,
            &msg.is_done
        ),
    )?;
    return Ok(msg.id);
}

pub fn insert_topic(
    connection: &Connection,
    topics: Vec<String>,
    message_id: Uuid,
) -> Result<(), IgrisError> {
    for assistant_topic in topics {
        let topic = MessageTopic {
            id: Uuid::new_v4(),
            message_id: message_id,
            topic: assistant_topic,
            timestamp: Local::now(),
        };
        connection.execute(
            "INSERT INTO message_topics (id, timestamp, message_id, topic) VALUES (?1, ?2, ?3, ?4)",
            (
                &topic.id.to_string(),
                &topic.timestamp.to_string(),
                &topic.message_id.to_string(),
                &topic.topic,
            ),
        )?;
    }
    return Ok(());
}

pub fn get_topics(connection: &Connection) -> Result<Vec<String>, IgrisError> {
    let mut query = connection.prepare("SELECT DISTINCT topic FROM message_topics")?;
    let topics = query
        .query_map([], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(topics)
}

pub fn get_last_session_with_messages(
    connection: &Connection,
) -> Result<Option<Session>, IgrisError> {
    let mut stmt = connection.prepare(
        "SELECT s.id, s.timestamp FROM sessions s
         WHERE EXISTS (SELECT 1 FROM messages m WHERE m.session_id = s.id)
         ORDER BY s.timestamp DESC LIMIT 1",
    )?;
    let session = stmt
        .query_row([], |row| {
            Ok(Session {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or(Uuid::nil()),
                timestamp: parse_db_timestamp(&row.get::<_, String>(1)?),
            })
        })
        .ok();
    Ok(session)
}

pub fn get_messages_by_session(
    connection: &Connection,
    session_id: &Uuid,
) -> Result<Vec<Message>, IgrisError> {
    let mut stmt = connection.prepare(
        "SELECT id, session_id, role, content, raw_json, timestamp, action, is_done FROM messages
         WHERE session_id = ?1 ORDER BY timestamp ASC",
    )?;
    let messages = stmt
        .query_map(params![session_id.to_string()], |row| {
            Ok(Message {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or(Uuid::nil()),
                session_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or(Uuid::nil()),
                role: row.get(2)?,
                content: row.get(3)?,
                raw_json: row.get::<_, Option<String>>(4)?,
                timestamp: parse_db_timestamp(&row.get::<_, String>(5)?),
                action: row.get::<_, Option<String>>(6)?,
                is_done: row.get::<_, bool>(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(messages)
}

pub fn search_messages(
    connection: &Connection,
    query: &str,
    limit: i64,
) -> Result<Vec<Message>, IgrisError> {
    let pattern = format!("%{}%", query);
    let mut stmt = connection.prepare(
        "SELECT id, session_id, role, content, timestamp, action, is_done 
         FROM messages 
         WHERE content LIKE ?1 
         ORDER BY timestamp DESC 
         LIMIT ?2",
    )?;
    let messages = stmt
        .query_map(params![pattern, limit], |row| {
            Ok(Message {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or(Uuid::nil()),
                session_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or(Uuid::nil()),
                role: row.get(2)?,
                content: row.get(3)?,
                raw_json: row.get::<_, Option<String>>(4)?,
                timestamp: parse_db_timestamp(&row.get::<_, String>(5)?),
                action: row.get::<_, Option<String>>(6)?,
                is_done: row.get::<_, bool>(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(messages)
}

pub fn get_session_context_with_limit(
    connection: &Connection,
    session_id: &str,
    token_limit: usize,
) -> Result<Vec<Message>, IgrisError> {
    let avg_tokens_per_message = 50;
    let max_messages = token_limit / avg_tokens_per_message;

    let mut stmt = connection.prepare(
        "SELECT id, session_id, role, content, action, is_done, timestamp 
         FROM messages 
         WHERE session_id = ?1 
         ORDER BY timestamp DESC 
         LIMIT ?2",
    )?;

    let messages = stmt
        .query_map(params![session_id, max_messages], |row| {
            Ok(Message {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or(Uuid::nil()),
                session_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or(Uuid::nil()),
                role: row.get(2)?,
                content: row.get(3)?,
                raw_json: row.get::<_, Option<String>>(4)?,
                action: row.get(5)?,
                is_done: row.get::<_, i32>(6)? != 0,
                timestamp: parse_db_timestamp(&row.get::<_, String>(7)?),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(messages)
}

pub fn trim_old_messages(
    connection: &Connection,
    session_id: &str,
    retention_days: i32,
) -> Result<usize, IgrisError> {
    let cutoff_date = chrono::Local::now() - chrono::Duration::days(retention_days as i64);
    let cutoff_str = cutoff_date.to_string();

    let affected = connection.execute(
        "DELETE FROM messages WHERE session_id = ?1 AND timestamp < ?2",
        rusqlite::params![session_id, &cutoff_str],
    )?;

    Ok(affected)
}

pub fn estimate_context_tokens(
    connection: &Connection,
    session_id: &str,
) -> Result<usize, IgrisError> {
    let mut stmt =
        connection.prepare("SELECT SUM(LENGTH(content)) FROM messages WHERE session_id = ?1")?;

    let total_chars: i32 = stmt.query_row(params![session_id], |row| {
        row.get::<_, Option<i32>>(0).map(|v| v.unwrap_or(0))
    })?;

    Ok((total_chars as usize) / 4)
}

pub fn get_context_paginated(
    connection: &Connection,
    session_id: &str,
    page: usize,
    page_size: usize,
) -> Result<Vec<Message>, IgrisError> {
    let offset = page * page_size;

    let mut stmt = connection.prepare(
        "SELECT id, session_id, role, content, action, is_done, timestamp 
         FROM messages 
         WHERE session_id = ?1 
         ORDER BY timestamp DESC 
         LIMIT ?2 OFFSET ?3",
    )?;

    let messages = stmt
        .query_map(rusqlite::params![session_id, page_size, offset], |row| {
            Ok(Message {
                id: Uuid::parse_str(&row.get::<_, String>(0)?).unwrap_or(Uuid::nil()),
                session_id: Uuid::parse_str(&row.get::<_, String>(1)?).unwrap_or(Uuid::nil()),
                role: row.get(2)?,
                content: row.get(3)?,
                raw_json: row.get::<_, Option<String>>(4)?,
                action: row.get(5)?,
                is_done: row.get::<_, i32>(6)? != 0,
                timestamp: parse_db_timestamp(&row.get::<_, String>(7)?),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(messages)
}
