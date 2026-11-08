//! Starboard tables: per-guild configuration and the messages that have been
//! starred.

use rusqlite::{Connection, OptionalExtension, params};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Config {
    pub channel_id: u64,
    pub threshold: u32,
}

/// A starred message's row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// The message posted to the starboard channel, once it exists.
    pub starboard_message_id: Option<u64>,
    /// Set while a starboard post is being sent, so a second star arriving in
    /// the meantime doesn't post it twice.
    pub posting: bool,
}

pub fn init(conn: &Connection) -> rusqlite::Result<()> {
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

    if !crate::db::column_exists(conn, "starred_messages", "posting")? {
        conn.execute(
            "ALTER TABLE starred_messages ADD COLUMN posting INTEGER NOT NULL DEFAULT 0",
            [],
        )?;
    }

    Ok(())
}

pub fn config(conn: &Connection, guild_id: u64) -> rusqlite::Result<Option<Config>> {
    conn.query_row(
        "SELECT channel_id, threshold FROM starboard_config WHERE guild_id = ?",
        [guild_id.cast_signed()],
        |row| {
            Ok(Config {
                channel_id: row.get::<_, i64>(0)?.cast_unsigned(),
                threshold: row.get::<_, i64>(1)?.try_into().unwrap_or(u32::MAX),
            })
        },
    )
    .optional()
}

pub fn set_config(conn: &Connection, guild_id: u64, config: Config) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO starboard_config (guild_id, channel_id, threshold) VALUES (?, ?, ?)",
        params![
            guild_id.cast_signed(),
            config.channel_id.cast_signed(),
            config.threshold
        ],
    )?;
    Ok(())
}

pub fn delete_config(conn: &Connection, guild_id: u64) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM starboard_config WHERE guild_id = ?",
        [guild_id.cast_signed()],
    )?;
    Ok(())
}

pub(super) fn entry(conn: &Connection, message_id: u64) -> rusqlite::Result<Option<Entry>> {
    conn.query_row(
        "SELECT starboard_message_id, posting FROM starred_messages WHERE message_id = ?",
        [message_id.cast_signed()],
        |row| {
            Ok(Entry {
                starboard_message_id: row.get::<_, Option<i64>>(0)?.map(i64::cast_unsigned),
                posting: row.get::<_, i64>(1)? != 0,
            })
        },
    )
    .optional()
}

/// Inserts a new entry already marked as posting.
pub(super) fn insert_posting(
    conn: &Connection,
    message_id: u64,
    guild_id: u64,
    channel_id: u64,
    star_count: u64,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO starred_messages (message_id, guild_id, channel_id, starboard_message_id, star_count, posting) VALUES (?, ?, ?, NULL, ?, 1)",
        params![
            message_id.cast_signed(),
            guild_id.cast_signed(),
            channel_id.cast_signed(),
            star_count.cast_signed()
        ],
    )?;
    Ok(())
}

/// Marks an existing, not-yet-posted entry as posting. Returns whether this
/// call claimed it.
pub(super) fn claim_posting(conn: &Connection, message_id: u64) -> rusqlite::Result<bool> {
    let rows = conn.execute(
        "UPDATE starred_messages SET posting = 1 WHERE message_id = ? AND posting = 0",
        [message_id.cast_signed()],
    )?;
    Ok(rows > 0)
}

pub(super) fn set_count(
    conn: &Connection,
    message_id: u64,
    star_count: u64,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE starred_messages SET star_count = ? WHERE message_id = ?",
        [star_count.cast_signed(), message_id.cast_signed()],
    )?;
    Ok(())
}

pub(super) fn set_posted(
    conn: &Connection,
    message_id: u64,
    starboard_message_id: u64,
    star_count: u64,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE starred_messages SET starboard_message_id = ?, posting = 0, star_count = ? WHERE message_id = ?",
        [
            starboard_message_id.cast_signed(),
            star_count.cast_signed(),
            message_id.cast_signed(),
        ],
    )?;
    Ok(())
}

pub(super) fn clear_posting(conn: &Connection, message_id: u64) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE starred_messages SET posting = 0 WHERE message_id = ?",
        [message_id.cast_signed()],
    )?;
    Ok(())
}

pub(super) fn delete_entry(conn: &Connection, message_id: u64) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM starred_messages WHERE message_id = ?",
        [message_id.cast_signed()],
    )?;
    Ok(())
}
