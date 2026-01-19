use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

// TODO: figure this out lol
pub fn get_data_dir() -> PathBuf {
    PathBuf::from("/var/lib/blahaj")
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

    Mutex::new(conn)
});
