use rusqlite::Connection;

#[test]
fn test_memory_skill_database_init() {
    let conn = Connection::open_in_memory().expect("Failed to open DB");
    
    let result = conn.execute_batch(
        "CREATE TABLE messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            action TEXT,
            is_done BOOLEAN DEFAULT 0
        );"
    );
    
    assert!(result.is_ok());
}

#[test]
fn test_insert_message() {
    let conn = Connection::open_in_memory().expect("Failed to open DB");
    
    conn.execute_batch(
        "CREATE TABLE messages (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            role TEXT NOT NULL,
            content TEXT NOT NULL,
            timestamp TEXT NOT NULL
        );"
    ).ok();
    
    let result = conn.execute(
        "INSERT INTO messages (id, session_id, role, content, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
        rusqlite::params!["1", "sess-1", "user", "test", "2026-06-14"],
    );
    
    assert!(result.is_ok());
}

#[test]
fn test_session_table() {
    let conn = Connection::open_in_memory().expect("Failed to open DB");
    
    conn.execute_batch(
        "CREATE TABLE sessions (
            id TEXT PRIMARY KEY,
            created_at TEXT NOT NULL
        );"
    ).ok();
    
    let result = conn.execute(
        "INSERT INTO sessions (id, created_at) VALUES (?1, ?2)",
        rusqlite::params!["sess-1", "2026-06-14"],
    );
    
    assert!(result.is_ok());
}
