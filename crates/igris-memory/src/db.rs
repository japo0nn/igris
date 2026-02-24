use crate::models::ChatMessage;
use chrono::Utc;
use rusqlite::{Connection, Result};

pub struct MemoryDb {
    conn: Connection,
}

impl MemoryDb {
    // Открывает или создаёт БД по указанному пути
    // Создаёт таблицы если их нет
    pub fn open(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS sessions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                started_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS chat_messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id INTEGER NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                FOREIGN KEY (session_id) REFERENCES sessions(id)
            );
        ",
        )?;

        Ok(Self { conn: conn })
    }

    pub fn save_message(&self, session_id: i64, msg: &ChatMessage) -> Result<()> {
        self.conn.execute("INSERT INTO chat_messages (session_id, role, content, timestamp) VALUES (?1, ?2, ?3, ?4)", (&session_id, &msg.role, &msg.content, &msg.timestamp, ),)?;
        Ok(())
    }

    pub fn get_session_message(&self, session_id: i64) -> Result<Vec<ChatMessage>> {
        let mut stmt = self
            .conn
            .prepare("SELECT role, content, timestamp FROM chat_messages WHERE session_id = ?1")?;

        let messages = stmt.query_map([session_id], |row| {
            Ok(ChatMessage {
                role: row.get(0)?,
                content: row.get(1)?,
                timestamp: row.get(2)?,
            })
        })?;
        
        messages.collect()
    }

    pub fn create_session(&self) -> Result<i64> {
        let date_time = Utc::now();

        self.conn.execute(
            "INSERT INTO sessions (started_at) VALUES (?1)",
            (&date_time.to_string(),),
        )?;
        let row_id = self.conn.last_insert_rowid();

        Ok(row_id)
    }
}
