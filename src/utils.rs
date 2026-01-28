use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

pub fn get_data_dir() -> PathBuf {
    crate::config::get().data_dir.clone()
}

pub static STARBOARD_DB: LazyLock<Mutex<Connection>> = LazyLock::new(|| {
    let db_path = get_data_dir().join("starboard.db");
    let conn = Connection::open(db_path).expect("Failed to open starboard database");

    conn.execute(
        "CREATE TABLE IF NOT EXISTS starred_messages (
            message_id INTEGER PRIMARY KEY,
            guild_id INTEGER NOT NULL,
            channel_id INTEGER NOT NULL,
            starboard_message_id INTEGER,
            star_count INTEGER NOT NULL DEFAULT 1,
            posting INTEGER NOT NULL DEFAULT 0,
            UNIQUE(message_id)
        )",
        [],
    )
    .expect("Failed to create starred_messages table");

    conn.execute(
        "CREATE TABLE IF NOT EXISTS starboard_config (
            guild_id INTEGER PRIMARY KEY,
            channel_id INTEGER NOT NULL,
            threshold INTEGER NOT NULL DEFAULT 3
        )",
        [],
    )
    .expect("Failed to create starboard_config table");

    ensure_starboard_schema(&conn).expect("Failed to migrate starboard schema");

    Mutex::new(conn)
});

fn ensure_starboard_schema(conn: &Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(starred_messages)")?;
    let column_names = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;

    if !column_names.iter().any(|name| name == "posting") {
        conn.execute(
            "ALTER TABLE starred_messages ADD COLUMN posting INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }

    Ok(())
}
