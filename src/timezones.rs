//! Store for `/timezone`: each user's timezone, per guild.

use rusqlite::{Connection, params};

pub fn init(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS user_timezones (
            guild_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            timezone TEXT NOT NULL,
            updated_at INTEGER NOT NULL,
            PRIMARY KEY (guild_id, user_id)
        )",
        [],
    )?;
    Ok(())
}

/// Sets (or replaces) `user_id`'s timezone in `guild_id`.
pub fn set(
    conn: &Connection,
    guild_id: u64,
    user_id: u64,
    timezone: &str,
    now: i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO user_timezones (guild_id, user_id, timezone, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(guild_id, user_id)
         DO UPDATE SET timezone = excluded.timezone, updated_at = excluded.updated_at",
        params![guild_id.cast_signed(), user_id.cast_signed(), timezone, now],
    )?;
    Ok(())
}

/// Removes `user_id`'s timezone in `guild_id`, returning whether there was one.
pub fn remove(conn: &Connection, guild_id: u64, user_id: u64) -> rusqlite::Result<bool> {
    let removed = conn.execute(
        "DELETE FROM user_timezones WHERE guild_id = ?1 AND user_id = ?2",
        params![guild_id.cast_signed(), user_id.cast_signed()],
    )?;
    Ok(removed > 0)
}

/// Every `(user_id, timezone)` set in `guild_id`.
pub fn list(conn: &Connection, guild_id: u64) -> rusqlite::Result<Vec<(u64, String)>> {
    conn.prepare("SELECT user_id, timezone FROM user_timezones WHERE guild_id = ?1")?
        .query_map([guild_id.cast_signed()], |row| {
            Ok((row.get::<_, i64>(0)?.cast_unsigned(), row.get(1)?))
        })?
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_replaces_and_remove_reports_whether_it_existed() {
        let conn = crate::db::test_connection();
        set(&conn, 10, 1, "Europe/London", 100).unwrap();
        set(&conn, 10, 1, "Asia/Tokyo", 200).unwrap();
        set(&conn, 20, 1, "America/New_York", 100).unwrap();

        assert_eq!(
            list(&conn, 10).unwrap(),
            vec![(1, "Asia/Tokyo".to_string())]
        );
        assert!(remove(&conn, 10, 1).unwrap());
        assert!(!remove(&conn, 10, 1).unwrap());
        assert_eq!(list(&conn, 10).unwrap(), Vec::<(u64, String)>::new());
        assert_eq!(list(&conn, 20).unwrap().len(), 1);
    }
}
