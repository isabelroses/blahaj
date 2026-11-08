//! Starboard rules: when a starred message is posted to, edited on, or removed
//! from the starboard channel.
//!
//! Callers report a star change with the message's current star count and get
//! back an [`Action`] to carry out on Discord, then report how that went with
//! [`finish_post`] or [`finish_delete`]. All bookkeeping, including the
//! `posting` flag that stops two quick stars from posting a message twice, lives
//! here, so it can be tested against an in-memory database.

pub mod store;

use rusqlite::Connection;

pub use store::Config;

/// What the caller should do on Discord after a star change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Post the message to the starboard channel, then call [`finish_post`].
    Post,
    /// Update the existing starboard post with this id.
    Edit(u64),
    /// Delete the starboard post with this id, then call [`finish_delete`].
    Delete(u64),
    Nothing,
}

/// A star was added to `message_id`, which now has `star_count` stars.
pub fn on_star_added(
    conn: &Connection,
    config: Config,
    message_id: u64,
    guild_id: u64,
    channel_id: u64,
    star_count: u64,
) -> rusqlite::Result<Action> {
    if star_count < u64::from(config.threshold) {
        return Ok(Action::Nothing);
    }

    let Some(entry) = store::entry(conn, message_id)? else {
        store::insert_posting(conn, message_id, guild_id, channel_id, star_count)?;
        return Ok(Action::Post);
    };

    store::set_count(conn, message_id, star_count)?;

    Ok(match entry.starboard_message_id {
        Some(id) => Action::Edit(id),
        None if store::claim_posting(conn, message_id)? => Action::Post,
        // Another star is already posting it.
        None => Action::Nothing,
    })
}

/// A star was removed from `message_id`, which now has `star_count` stars.
pub fn on_star_removed(
    conn: &Connection,
    config: Config,
    message_id: u64,
    star_count: u64,
) -> rusqlite::Result<Action> {
    let Some(entry) = store::entry(conn, message_id)? else {
        return Ok(Action::Nothing);
    };

    if star_count >= u64::from(config.threshold) {
        store::set_count(conn, message_id, star_count)?;
        return Ok(entry
            .starboard_message_id
            .map_or(Action::Nothing, Action::Edit));
    }

    match entry.starboard_message_id {
        Some(id) => return Ok(Action::Delete(id)),
        None if entry.posting => store::set_count(conn, message_id, star_count)?,
        None => store::delete_entry(conn, message_id)?,
    }
    Ok(Action::Nothing)
}

/// Records the result of an [`Action::Post`]: the id of the starboard post, or
/// `None` if sending failed (so a later star can try again).
pub fn finish_post(
    conn: &Connection,
    message_id: u64,
    starboard_message_id: Option<u64>,
    star_count: u64,
) -> rusqlite::Result<()> {
    match starboard_message_id {
        Some(id) => store::set_posted(conn, message_id, id, star_count),
        None => store::clear_posting(conn, message_id),
    }
}

/// Records the result of an [`Action::Delete`]. If the starboard post couldn't
/// be deleted, the entry is kept so a later change can try again.
pub fn finish_delete(
    conn: &Connection,
    message_id: u64,
    deleted: bool,
    star_count: u64,
) -> rusqlite::Result<()> {
    if deleted {
        store::delete_entry(conn, message_id)
    } else {
        store::set_count(conn, message_id, star_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: Config = Config {
        channel_id: 900,
        threshold: 2,
    };
    const MSG: u64 = 1;

    fn add(conn: &Connection, count: u64) -> Action {
        on_star_added(conn, CONFIG, MSG, 10, 20, count).unwrap()
    }

    fn remove(conn: &Connection, count: u64) -> Action {
        on_star_removed(conn, CONFIG, MSG, count).unwrap()
    }

    #[test]
    fn below_threshold_does_nothing_and_stores_nothing() {
        let conn = crate::db::test_connection();
        assert_eq!(add(&conn, 1), Action::Nothing);
        assert_eq!(store::entry(&conn, MSG).unwrap(), None);
    }

    #[test]
    fn second_star_while_posting_does_not_post_twice() {
        let conn = crate::db::test_connection();
        assert_eq!(add(&conn, 2), Action::Post);
        // A third star arrives before the post has been sent.
        assert_eq!(add(&conn, 3), Action::Nothing);

        finish_post(&conn, MSG, Some(55), 3).unwrap();
        assert_eq!(add(&conn, 4), Action::Edit(55));
    }

    #[test]
    fn failed_post_can_be_retried() {
        let conn = crate::db::test_connection();
        assert_eq!(add(&conn, 2), Action::Post);
        finish_post(&conn, MSG, None, 2).unwrap();
        assert_eq!(add(&conn, 3), Action::Post);
    }

    #[test]
    fn dropping_below_threshold_deletes_the_post() {
        let conn = crate::db::test_connection();
        add(&conn, 2);
        finish_post(&conn, MSG, Some(55), 2).unwrap();

        assert_eq!(remove(&conn, 2), Action::Edit(55));
        assert_eq!(remove(&conn, 1), Action::Delete(55));

        finish_delete(&conn, MSG, true, 1).unwrap();
        assert_eq!(store::entry(&conn, MSG).unwrap(), None);
    }

    #[test]
    fn failed_delete_keeps_the_entry() {
        let conn = crate::db::test_connection();
        add(&conn, 2);
        finish_post(&conn, MSG, Some(55), 2).unwrap();
        assert_eq!(remove(&conn, 1), Action::Delete(55));

        finish_delete(&conn, MSG, false, 1).unwrap();
        assert_eq!(remove(&conn, 0), Action::Delete(55));
    }

    #[test]
    fn removing_while_posting_keeps_the_entry() {
        let conn = crate::db::test_connection();
        assert_eq!(add(&conn, 2), Action::Post);
        assert_eq!(remove(&conn, 1), Action::Nothing);
        assert!(store::entry(&conn, MSG).unwrap().unwrap().posting);
    }

    #[test]
    fn removing_from_unposted_entry_deletes_it() {
        let conn = crate::db::test_connection();
        add(&conn, 2);
        finish_post(&conn, MSG, None, 2).unwrap();
        assert_eq!(remove(&conn, 1), Action::Nothing);
        assert_eq!(store::entry(&conn, MSG).unwrap(), None);
    }
}
