use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

pub fn get_data_dir() -> PathBuf {
    crate::config::get().data_dir.clone()
}

/// The single application database. Every feature stores its state here in its
/// own table(s); the large, auto-generated nixpkgs `packages.db` is kept
/// separate since it is rebuilt wholesale.
pub static DB: LazyLock<Mutex<Connection>> = LazyLock::new(|| {
    let db_path = get_data_dir().join("blahaj.db");
    let conn = Connection::open(db_path).expect("Failed to open database");

    init_schema(&conn).expect("Failed to initialise database schema");
    migrate_legacy_databases(&conn).expect("Failed to migrate legacy databases");

    Mutex::new(conn)
});

/// The pre-consolidation database files and the tables they held. On first run
/// after the switch to a single database, any of these still present are
/// imported into `blahaj.db` and then renamed to `<name>.migrated` so the
/// import happens exactly once.
const LEGACY_DATABASES: &[(&str, &[&str])] = &[
    ("starboard.db", &["starred_messages", "starboard_config"]),
    ("tracked_prs.db", &["tracked_prs"]),
    ("avatar_emojis.db", &["avatar_emojis"]),
    ("color_roles.db", &["color_roles"]),
    (
        "relationship.db",
        &[
            "relationships",
            "relationship_members",
            "relationship_invites",
        ],
    ),
];

fn migrate_legacy_databases(conn: &Connection) -> rusqlite::Result<()> {
    let dir = get_data_dir();

    for (file, tables) in LEGACY_DATABASES {
        let path = dir.join(file);
        if !path.exists() {
            continue;
        }

        conn.execute(
            "ATTACH DATABASE ? AS legacy",
            [path.to_string_lossy().as_ref()],
        )?;

        for table in *tables {
            copy_table(conn, table)?;
        }

        conn.execute("DETACH DATABASE legacy", [])?;

        // Keep the old file around (renamed) rather than deleting it, so the
        // import never repeats and the original data isn't lost.
        let _ = std::fs::rename(&path, dir.join(format!("{file}.migrated")));
    }

    Ok(())
}

/// Copy every row of `table` from the attached `legacy` database into the
/// matching table in the main database, restricting to the columns the two
/// schemas have in common. `INSERT OR IGNORE` skips rows that already exist.
fn copy_table(conn: &Connection, table: &str) -> rusqlite::Result<()> {
    let legacy_cols = table_columns(conn, &format!("legacy.table_info({table})"))?;
    let main_cols = table_columns(conn, &format!("table_info({table})"))?;

    let cols: Vec<&str> = legacy_cols
        .iter()
        .filter(|c| main_cols.contains(c))
        .map(String::as_str)
        .collect();

    if cols.is_empty() {
        return Ok(());
    }

    let cols = cols.join(", ");
    conn.execute(
        &format!("INSERT OR IGNORE INTO main.{table} ({cols}) SELECT {cols} FROM legacy.{table}"),
        [],
    )?;

    Ok(())
}

fn init_schema(conn: &Connection) -> rusqlite::Result<()> {
    init_starboard(conn)?;
    init_tracked_prs(conn)?;
    init_avatar_emojis(conn)?;
    init_color_roles(conn)?;
    init_relationships(conn)?;
    Ok(())
}

fn init_starboard(conn: &Connection) -> rusqlite::Result<()> {
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
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS starboard_config (
            guild_id INTEGER PRIMARY KEY,
            channel_id INTEGER NOT NULL,
            threshold INTEGER NOT NULL DEFAULT 3
        )",
        [],
    )?;

    if !column_exists(conn, "starred_messages", "posting")? {
        conn.execute(
            "ALTER TABLE starred_messages ADD COLUMN posting INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }

    Ok(())
}

fn init_tracked_prs(conn: &Connection) -> rusqlite::Result<()> {
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
    )?;

    if !column_exists(conn, "tracked_prs", "target_branch")? {
        conn.execute(
            "ALTER TABLE tracked_prs ADD COLUMN target_branch TEXT NOT NULL DEFAULT 'nixpkgs-unstable'",
            [],
        )?;
    }

    Ok(())
}

fn init_avatar_emojis(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS avatar_emojis (
            user_id INTEGER NOT NULL,
            guild_id INTEGER NOT NULL,
            emoji_id INTEGER NOT NULL,
            emoji_name TEXT NOT NULL,
            PRIMARY KEY (user_id, guild_id)
        )",
        [],
    )?;

    Ok(())
}

fn init_color_roles(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS color_roles (
            user_id INTEGER PRIMARY KEY,
            guild_id INTEGER NOT NULL,
            role_id INTEGER NOT NULL,
            role_name TEXT NOT NULL
        )",
        [],
    )?;

    Ok(())
}

fn init_relationships(conn: &Connection) -> rusqlite::Result<()> {
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
    )?;

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
    )?;

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
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_relationships_guild_type_status
         ON relationships (guild_id, relationship_type, status)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_members_user_active
         ON relationship_members (user_id, left_at)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_invites_invitee_status
         ON relationship_invites (invitee_id, status)",
        [],
    )?;

    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_invites_relationship_status
         ON relationship_invites (relationship_id, status)",
        [],
    )?;

    if !column_exists(conn, "relationships", "emoji")? {
        conn.execute("ALTER TABLE relationships ADD COLUMN emoji TEXT", [])?;
    }

    if !column_exists(conn, "relationships", "description")? {
        conn.execute("ALTER TABLE relationships ADD COLUMN description TEXT", [])?;
    }

    Ok(())
}

/// Whether `table` already has a column named `column`.
fn column_exists(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let columns = table_columns(conn, &format!("table_info({table})"))?;
    Ok(columns.iter().any(|name| name == column))
}

/// The column names reported by a `table_info` pragma. `pragma_target` is the
/// pragma call without the leading `PRAGMA`, e.g. `table_info(foo)` or
/// `legacy.table_info(foo)` for an attached database. Returns an empty list if
/// the table doesn't exist.
fn table_columns(conn: &Connection, pragma_target: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(&format!("PRAGMA {pragma_target}"))?;
    let columns = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(Result::ok)
        .collect();
    Ok(columns)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_table_imports_only_common_columns_and_skips_conflicts() {
        let legacy_path =
            std::env::temp_dir().join(format!("blahaj-migrate-test-{}.db", std::process::id()));
        let _ = std::fs::remove_file(&legacy_path);

        // A legacy database with an extra, since-removed column.
        let legacy = Connection::open(&legacy_path).unwrap();
        legacy
            .execute(
                "CREATE TABLE color_roles (
                    user_id INTEGER PRIMARY KEY,
                    guild_id INTEGER NOT NULL,
                    role_id INTEGER NOT NULL,
                    role_name TEXT NOT NULL,
                    obsolete TEXT
                )",
                [],
            )
            .unwrap();
        legacy
            .execute(
                "INSERT INTO color_roles VALUES (1, 10, 100, 'red', 'x'), (2, 20, 200, 'blue', 'y')",
                [],
            )
            .unwrap();
        drop(legacy);

        // The new unified database, already holding a row that conflicts on the
        // primary key.
        let conn = Connection::open_in_memory().unwrap();
        init_color_roles(&conn).unwrap();
        conn.execute("INSERT INTO color_roles VALUES (2, 99, 999, 'keep-me')", [])
            .unwrap();

        conn.execute(
            "ATTACH DATABASE ? AS legacy",
            [legacy_path.to_string_lossy().as_ref()],
        )
        .unwrap();
        copy_table(&conn, "color_roles").unwrap();
        conn.execute("DETACH DATABASE legacy", []).unwrap();

        let mut rows: Vec<(i64, String)> = conn
            .prepare("SELECT user_id, role_name FROM color_roles ORDER BY user_id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        rows.sort();

        // Row 1 imported; row 2 kept its existing value (INSERT OR IGNORE).
        assert_eq!(
            rows,
            vec![(1, "red".to_string()), (2, "keep-me".to_string())]
        );

        let _ = std::fs::remove_file(&legacy_path);
    }
}
