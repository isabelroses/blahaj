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

pub static TRACKED_PRS_DB: LazyLock<Mutex<Connection>> = LazyLock::new(|| {
    let db_path = get_data_dir().join("tracked_prs.db");
    let conn = Connection::open(db_path).expect("Failed to open tracked PRs database");

    conn.execute(
        "CREATE TABLE IF NOT EXISTS tracked_prs (
            pr_number INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            channel_id INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            target_branch TEXT NOT NULL DEFAULT 'nixpkgs-unstable',
            PRIMARY KEY (pr_number, user_id)
        )",
        [],
    )
    .expect("Failed to create tracked_prs table");

    ensure_tracked_prs_schema(&conn).expect("Failed to migrate tracked_prs schema");

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

fn ensure_tracked_prs_schema(conn: &Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(tracked_prs)")?;
    let column_names = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;

    if !column_names.iter().any(|name| name == "target_branch") {
        conn.execute(
            "ALTER TABLE tracked_prs ADD COLUMN target_branch TEXT NOT NULL DEFAULT 'nixpkgs-unstable'",
            [],
        )?;
    }

    Ok(())
}
