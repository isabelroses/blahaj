//! The application database: one `SQLite` file shared by every feature.
//!
//! Each feature owns its tables in its own store module and takes a
//! `&Connection`, so tests can hand it an in-memory database. This module
//! only opens the shared connection, runs every store's `init`, and imports
//! the pre-consolidation database files once.

use rusqlite::Connection;
use std::sync::{LazyLock, Mutex, PoisonError};

static DB: LazyLock<Mutex<Connection>> = LazyLock::new(|| {
    let conn = Connection::open(crate::config::data_dir().join("blahaj.db"))
        .expect("Failed to open database");

    init(&conn).expect("Failed to initialise database schema");
    migrate_legacy_databases(&conn).expect("Failed to migrate legacy databases");

    Mutex::new(conn)
});

/// Runs `f` with the shared connection. `SQLite` calls block, so this moves
/// the current task off the async worker thread while `f` runs.
pub fn with<T>(f: impl FnOnce(&Connection) -> T) -> T {
    tokio::task::block_in_place(|| f(&DB.lock().unwrap_or_else(PoisonError::into_inner)))
}

/// Creates or upgrades every feature's tables.
pub fn init(conn: &Connection) -> rusqlite::Result<()> {
    crate::starboard::store::init(conn)?;
    crate::nixpkgs::tracking::init(conn)?;
    crate::avatarsync::store::init(conn)?;
    crate::color_roles::init(conn)?;
    crate::timezones::init(conn)?;
    Ok(())
}

/// The pre-consolidation database files and the tables they held. On first run
/// after the switch to a single database, any of these still present are
/// imported into `blahaj.db` and then renamed to `<name>.migrated` so the
/// import happens exactly once.
const LEGACY_DATABASES: &[(&str, &[&str])] = &[
    ("starboard.db", &["starred_messages", "starboard_config"]),
    ("tracked_prs.db", &["tracked_prs"]),
    ("avatar_emojis.db", &["avatar_emojis"]),
    ("color_roles.db", &["color_roles"]),
];

fn migrate_legacy_databases(conn: &Connection) -> rusqlite::Result<()> {
    let dir = crate::config::data_dir();

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

/// Whether `table` already has a column named `column`. Stores use this to add
/// columns to tables created by older versions.
pub fn column_exists(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
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

/// An in-memory database with every feature's tables, for tests.
#[cfg(test)]
pub fn test_connection() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    init(&conn).unwrap();
    conn
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
        let conn = test_connection();
        conn.execute("INSERT INTO color_roles VALUES (2, 99, 999, 'keep-me')", [])
            .unwrap();

        conn.execute(
            "ATTACH DATABASE ? AS legacy",
            [legacy_path.to_string_lossy().as_ref()],
        )
        .unwrap();
        copy_table(&conn, "color_roles").unwrap();
        conn.execute("DETACH DATABASE legacy", []).unwrap();

        let rows: Vec<(i64, String)> = conn
            .prepare("SELECT user_id, role_name FROM color_roles ORDER BY user_id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect();

        // Row 1 imported; row 2 kept its existing value (INSERT OR IGNORE).
        assert_eq!(
            rows,
            vec![(1, "red".to_string()), (2, "keep-me".to_string())]
        );

        let _ = std::fs::remove_file(&legacy_path);
    }
}
