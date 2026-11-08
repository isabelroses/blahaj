//! Which guild emoji was made from which user's avatar.

use rusqlite::{Connection, OptionalExtension, params};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AvatarEmoji {
    pub guild_id: u64,
    pub emoji_id: u64,
    pub emoji_name: String,
    /// The avatar the emoji was made from, to skip refreshes that change nothing.
    pub avatar_url: Option<String>,
}

pub fn init(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS avatar_emojis (
            user_id INTEGER NOT NULL,
            guild_id INTEGER NOT NULL,
            emoji_id INTEGER NOT NULL,
            emoji_name TEXT NOT NULL,
            avatar_url TEXT,
            PRIMARY KEY (user_id, guild_id)
        )",
        [],
    )?;

    if !crate::db::column_exists(conn, "avatar_emojis", "avatar_url")? {
        conn.execute("ALTER TABLE avatar_emojis ADD COLUMN avatar_url TEXT", [])?;
    }

    Ok(())
}

fn from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AvatarEmoji> {
    Ok(AvatarEmoji {
        guild_id: row.get::<_, i64>(0)?.cast_unsigned(),
        emoji_id: row.get::<_, i64>(1)?.cast_unsigned(),
        emoji_name: row.get(2)?,
        avatar_url: row.get(3)?,
    })
}

pub fn get(
    conn: &Connection,
    user_id: u64,
    guild_id: u64,
) -> rusqlite::Result<Option<AvatarEmoji>> {
    conn.query_row(
        "SELECT guild_id, emoji_id, emoji_name, avatar_url FROM avatar_emojis WHERE user_id = ? AND guild_id = ?",
        params![user_id.cast_signed(), guild_id.cast_signed()],
        from_row,
    )
    .optional()
}

pub fn for_user(conn: &Connection, user_id: u64) -> rusqlite::Result<Vec<AvatarEmoji>> {
    conn.prepare(
        "SELECT guild_id, emoji_id, emoji_name, avatar_url FROM avatar_emojis WHERE user_id = ?",
    )?
    .query_map([user_id.cast_signed()], from_row)?
    .collect()
}

pub fn save(conn: &Connection, user_id: u64, emoji: &AvatarEmoji) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO avatar_emojis (user_id, guild_id, emoji_id, emoji_name, avatar_url) VALUES (?, ?, ?, ?, ?)",
        params![
            user_id.cast_signed(),
            emoji.guild_id.cast_signed(),
            emoji.emoji_id.cast_signed(),
            emoji.emoji_name,
            emoji.avatar_url,
        ],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, user_id: u64, guild_id: u64) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM avatar_emojis WHERE user_id = ? AND guild_id = ?",
        params![user_id.cast_signed(), guild_id.cast_signed()],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn emoji(guild_id: u64, emoji_id: u64) -> AvatarEmoji {
        AvatarEmoji {
            guild_id,
            emoji_id,
            emoji_name: "blahaj".to_string(),
            avatar_url: Some("https://cdn/a.png".to_string()),
        }
    }

    #[test]
    fn save_replaces_and_lists_per_user() {
        let conn = crate::db::test_connection();
        save(&conn, 1, &emoji(10, 100)).unwrap();
        save(&conn, 1, &emoji(20, 200)).unwrap();
        save(&conn, 1, &emoji(10, 101)).unwrap(); // replaces guild 10's emoji
        save(&conn, 2, &emoji(10, 300)).unwrap();

        assert_eq!(get(&conn, 1, 10).unwrap(), Some(emoji(10, 101)));
        let mut guilds: Vec<u64> = for_user(&conn, 1)
            .unwrap()
            .into_iter()
            .map(|e| e.guild_id)
            .collect();
        guilds.sort_unstable();
        assert_eq!(guilds, vec![10, 20]);

        delete(&conn, 1, 10).unwrap();
        assert_eq!(get(&conn, 1, 10).unwrap(), None);
    }
}
