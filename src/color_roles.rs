//! Store for `/color_me` roles: which role in a guild belongs to which user.

use rusqlite::{Connection, OptionalExtension, params};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColorRole {
    pub role_id: u64,
    pub name: String,
}

pub fn init(conn: &Connection) -> rusqlite::Result<()> {
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

pub fn get(conn: &Connection, user_id: u64, guild_id: u64) -> rusqlite::Result<Option<ColorRole>> {
    conn.query_row(
        "SELECT role_id, role_name FROM color_roles WHERE user_id = ? AND guild_id = ?",
        params![user_id.cast_signed(), guild_id.cast_signed()],
        |row| {
            Ok(ColorRole {
                role_id: row.get::<_, i64>(0)?.cast_unsigned(),
                name: row.get(1)?,
            })
        },
    )
    .optional()
}

pub fn save(
    conn: &Connection,
    user_id: u64,
    guild_id: u64,
    role_id: u64,
    name: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO color_roles (user_id, guild_id, role_id, role_name) VALUES (?, ?, ?, ?)",
        params![user_id.cast_signed(), guild_id.cast_signed(), role_id.cast_signed(), name],
    )?;
    Ok(())
}

pub fn rename(conn: &Connection, user_id: u64, guild_id: u64, name: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE color_roles SET role_name = ? WHERE user_id = ? AND guild_id = ?",
        params![name, user_id.cast_signed(), guild_id.cast_signed()],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, user_id: u64, guild_id: u64) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM color_roles WHERE user_id = ? AND guild_id = ?",
        params![user_id.cast_signed(), guild_id.cast_signed()],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_rename_delete_round_trip() {
        let conn = crate::db::test_connection();
        assert_eq!(get(&conn, 1, 10).unwrap(), None);

        save(&conn, 1, 10, 100, "red").unwrap();
        rename(&conn, 1, 10, "crimson").unwrap();
        assert_eq!(
            get(&conn, 1, 10).unwrap(),
            Some(ColorRole {
                role_id: 100,
                name: "crimson".to_string()
            })
        );
        // Lookups are per guild.
        assert_eq!(get(&conn, 1, 20).unwrap(), None);

        delete(&conn, 1, 10).unwrap();
        assert_eq!(get(&conn, 1, 10).unwrap(), None);
    }
}
