use rusqlite::Connection;
use std::sync::{LazyLock, Mutex};

pub static RELATIONSHIP_DB: LazyLock<Mutex<Connection>> = LazyLock::new(|| {
    let db_path = crate::utils::get_data_dir().join("relationship.db");
    let conn = Connection::open(db_path).expect("Failed to open relationship database");

    conn.execute(
        "CREATE TABLE IF NOT EXISTS relationships (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            guild_id INTEGER NOT NULL,
            relationship_type TEXT NOT NULL,
            emoji TEXT,
            description TEXT,
            status TEXT NOT NULL CHECK(status IN ('active', 'ended')) DEFAULT 'active',
            created_by INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            ended_at INTEGER
        )",
        [],
    )
    .expect("Failed to create relationships table");

    conn.execute(
        "CREATE TABLE IF NOT EXISTS relationship_members (
            relationship_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            joined_at INTEGER NOT NULL,
            left_at INTEGER,
            PRIMARY KEY (relationship_id, user_id),
            FOREIGN KEY (relationship_id) REFERENCES relationships(id)
        )",
        [],
    )
    .expect("Failed to create relationship_members table");

    conn.execute(
        "CREATE TABLE IF NOT EXISTS relationship_invites (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            relationship_id INTEGER NOT NULL,
            inviter_id INTEGER NOT NULL,
            invitee_id INTEGER NOT NULL,
            status TEXT NOT NULL CHECK(status IN ('pending', 'accepted', 'declined', 'cancelled')) DEFAULT 'pending',
            created_at INTEGER NOT NULL,
            responded_at INTEGER,
            FOREIGN KEY (relationship_id) REFERENCES relationships(id)
        )",
        [],
    )
    .expect("Failed to create relationship_invites table");

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_relationships_guild_type_status
         ON relationships (guild_id, relationship_type, status)",
        [],
    )
    .expect("Failed to create relationships index");

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_members_user_active
         ON relationship_members (user_id, left_at)",
        [],
    )
    .expect("Failed to create members index");

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_invites_invitee_status
         ON relationship_invites (invitee_id, status)",
        [],
    )
    .expect("Failed to create invites invitee index");

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_invites_relationship_status
         ON relationship_invites (relationship_id, status)",
        [],
    )
    .expect("Failed to create invites relationship index");

    ensure_relationship_schema(&conn).expect("Failed to migrate relationship schema");

    Mutex::new(conn)
});

fn ensure_relationship_schema(conn: &Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("PRAGMA table_info(relationships)")?;
    let column_names = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<Result<Vec<_>, _>>()?;

    if !column_names.iter().any(|name| name == "emoji") {
        conn.execute("ALTER TABLE relationships ADD COLUMN emoji TEXT", [])?;
    }

    if !column_names.iter().any(|name| name == "description") {
        conn.execute("ALTER TABLE relationships ADD COLUMN description TEXT", [])?;
    }

    Ok(())
}
