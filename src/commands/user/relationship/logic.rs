use color_eyre::eyre::Result;
use rusqlite::{Connection, params};

#[derive(Debug)]
pub struct MakeResolution {
    pub relationship_id: i64,
    pub created_new_group: bool,
}

pub fn normalize_relationship_type(raw: &str) -> Result<String> {
    let trimmed = raw.trim().to_lowercase();
    if trimmed.is_empty() {
        return Err(color_eyre::eyre::eyre!(
            "Relationship type cannot be empty."
        ));
    }

    let collapsed = trimmed.split_whitespace().collect::<Vec<_>>().join("-");
    if collapsed.len() < 2 || collapsed.len() > 32 {
        return Err(color_eyre::eyre::eyre!(
            "Relationship type must be between 2 and 32 characters."
        ));
    }

    if !collapsed
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(color_eyre::eyre::eyre!(
            "Relationship type can only contain letters, numbers, spaces, and dashes."
        ));
    }

    Ok(collapsed)
}

pub fn resolve_relationship_for_make(
    conn: &Connection,
    guild_id: u64,
    caller_id: u64,
    relationship_type: &str,
    explicit_relationship_id: Option<i64>,
) -> Result<std::result::Result<MakeResolution, String>> {
    if let Some(explicit_id) = explicit_relationship_id {
        let exists_and_allowed = conn
            .query_row(
                "SELECT 1
                 FROM relationships r
                 INNER JOIN relationship_members m ON m.relationship_id = r.id
                 WHERE r.id = ?
                   AND r.guild_id = ?
                   AND r.relationship_type = ?
                   AND r.status = 'active'
                   AND m.user_id = ?
                   AND m.left_at IS NULL",
                params![
                    explicit_id,
                    guild_id.cast_signed(),
                    relationship_type,
                    caller_id.cast_signed(),
                ],
                |_| Ok(()),
            )
            .ok()
            .is_some();

        if exists_and_allowed {
            return Ok(Ok(MakeResolution {
                relationship_id: explicit_id,
                created_new_group: false,
            }));
        }

        return Ok(Err(
            "That relationship ID is not an active relationship of this type you are in."
                .to_string(),
        ));
    }

    let existing =
        caller_active_relationships_by_type(conn, guild_id, relationship_type, caller_id)?;

    if existing.len() > 1 {
        let list = existing
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");
        return Ok(Err(format!(
            "You have multiple `{relationship_type}` relationships: {list}. Please pass `relationship_id` to choose one."
        )));
    }

    if let Some(id) = existing.first() {
        return Ok(Ok(MakeResolution {
            relationship_id: *id,
            created_new_group: false,
        }));
    }

    let new_id = create_relationship(conn, guild_id, caller_id, relationship_type)?;
    add_active_member(conn, new_id, caller_id)?;

    Ok(Ok(MakeResolution {
        relationship_id: new_id,
        created_new_group: true,
    }))
}

pub fn try_create_invite(
    conn: &Connection,
    relationship_id: i64,
    inviter_id: u64,
    invitee_id: u64,
) -> Result<Option<String>> {
    let already_member = conn
        .query_row(
            "SELECT 1 FROM relationship_members
             WHERE relationship_id = ? AND user_id = ? AND left_at IS NULL",
            params![relationship_id, invitee_id.cast_signed()],
            |_| Ok(()),
        )
        .ok()
        .is_some();

    if already_member {
        return Ok(Some(format!(
            "<@{invitee_id}> is already in relationship #{relationship_id}."
        )));
    }

    let pending_exists = conn
        .query_row(
            "SELECT 1 FROM relationship_invites
             WHERE relationship_id = ? AND invitee_id = ? AND status = 'pending'",
            params![relationship_id, invitee_id.cast_signed()],
            |_| Ok(()),
        )
        .ok()
        .is_some();

    if pending_exists {
        return Ok(Some(format!(
            "There is already a pending invite for <@{invitee_id}> in relationship #{relationship_id}."
        )));
    }

    conn.execute(
        "INSERT INTO relationship_invites (relationship_id, inviter_id, invitee_id, status, created_at)
         VALUES (?, ?, ?, 'pending', ?)",
        params![
            relationship_id,
            inviter_id.cast_signed(),
            invitee_id.cast_signed(),
            now_ts(),
        ],
    )?;

    Ok(None)
}

pub fn has_pending_invite(
    conn: &Connection,
    relationship_id: i64,
    invitee_id: u64,
    guild_id: u64,
) -> bool {
    conn.query_row(
        "SELECT 1
         FROM relationship_invites i
         INNER JOIN relationships r ON r.id = i.relationship_id
         WHERE i.relationship_id = ?
           AND i.invitee_id = ?
           AND i.status = 'pending'
           AND r.guild_id = ?
           AND r.status = 'active'",
        params![
            relationship_id,
            invitee_id.cast_signed(),
            guild_id.cast_signed(),
        ],
        |_| Ok(()),
    )
    .ok()
    .is_some()
}

pub fn accept_invite(conn: &Connection, relationship_id: i64, invitee_id: u64) -> Result<()> {
    add_active_member(conn, relationship_id, invitee_id)?;
    conn.execute(
        "UPDATE relationship_invites
         SET status = 'accepted', responded_at = ?
         WHERE relationship_id = ? AND invitee_id = ? AND status = 'pending'",
        params![now_ts(), relationship_id, invitee_id.cast_signed()],
    )?;
    Ok(())
}

pub fn decline_invite(
    conn: &Connection,
    relationship_id: i64,
    invitee_id: u64,
    guild_id: u64,
) -> Result<usize> {
    Ok(conn.execute(
        "UPDATE relationship_invites
         SET status = 'declined', responded_at = ?
         WHERE relationship_id = ?
           AND invitee_id = ?
           AND status = 'pending'
           AND EXISTS (
                SELECT 1 FROM relationships r
                WHERE r.id = relationship_invites.relationship_id
                  AND r.guild_id = ?
           )",
        params![
            now_ts(),
            relationship_id,
            invitee_id.cast_signed(),
            guild_id.cast_signed(),
        ],
    )?)
}

pub fn shared_relationship_ids(
    conn: &Connection,
    guild_id: u64,
    relationship_type: &str,
    user_a: u64,
    user_b: u64,
) -> Result<Vec<i64>> {
    shared_relationships_by_type(conn, guild_id, relationship_type, user_a, user_b)
}

pub fn leave_relationship(conn: &Connection, relationship_id: i64, user_id: u64) -> Result<bool> {
    let updated = conn.execute(
        "UPDATE relationship_members
         SET left_at = ?
         WHERE relationship_id = ? AND user_id = ? AND left_at IS NULL",
        params![now_ts(), relationship_id, user_id.cast_signed()],
    )?;

    if updated == 0 {
        return Ok(false);
    }

    maybe_end_relationship(conn, relationship_id)
}

pub fn is_active_member(
    conn: &Connection,
    relationship_id: i64,
    guild_id: u64,
    user_id: u64,
) -> bool {
    conn.query_row(
        "SELECT 1
         FROM relationships r
         INNER JOIN relationship_members m ON m.relationship_id = r.id
         WHERE r.id = ?
           AND r.guild_id = ?
           AND r.status = 'active'
           AND m.user_id = ?
           AND m.left_at IS NULL",
        params![
            relationship_id,
            guild_id.cast_signed(),
            user_id.cast_signed()
        ],
        |_| Ok(()),
    )
    .ok()
    .is_some()
}

pub fn list_active_relationships_for_user(
    conn: &Connection,
    guild_id: u64,
    user_id: u64,
) -> Result<Vec<(i64, String)>> {
    let mut stmt = conn.prepare(
        "SELECT r.id, r.relationship_type
         FROM relationships r
         INNER JOIN relationship_members m ON m.relationship_id = r.id
         WHERE r.guild_id = ?
           AND r.status = 'active'
           AND m.user_id = ?
           AND m.left_at IS NULL
         ORDER BY r.relationship_type ASC, r.id ASC",
    )?;

    let rows = stmt.query_map(
        params![guild_id.cast_signed(), user_id.cast_signed()],
        |row| {
            let relationship_id: i64 = row.get(0)?;
            let relationship_type: String = row.get(1)?;
            Ok((relationship_id, relationship_type))
        },
    )?;

    let mut values = Vec::new();
    for row in rows {
        values.push(row?);
    }

    Ok(values)
}

pub fn active_member_ids(conn: &Connection, relationship_id: i64) -> Result<Vec<u64>> {
    let mut stmt = conn.prepare(
        "SELECT user_id
         FROM relationship_members
         WHERE relationship_id = ? AND left_at IS NULL
         ORDER BY user_id ASC",
    )?;

    let rows = stmt.query_map([relationship_id], |row| {
        let user_id: i64 = row.get(0)?;
        Ok(user_id.cast_unsigned())
    })?;

    let mut members = Vec::new();
    for row in rows {
        members.push(row?);
    }

    Ok(members)
}

fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

fn create_relationship(
    conn: &Connection,
    guild_id: u64,
    created_by: u64,
    relationship_type: &str,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO relationships (guild_id, relationship_type, status, created_by, created_at)
         VALUES (?, ?, 'active', ?, ?)",
        params![
            guild_id.cast_signed(),
            relationship_type,
            created_by.cast_signed(),
            now_ts(),
        ],
    )?;

    Ok(conn.last_insert_rowid())
}

fn add_active_member(conn: &Connection, relationship_id: i64, user_id: u64) -> Result<()> {
    conn.execute(
        "INSERT INTO relationship_members (relationship_id, user_id, joined_at, left_at)
         VALUES (?, ?, ?, NULL)
         ON CONFLICT(relationship_id, user_id)
         DO UPDATE SET joined_at = excluded.joined_at, left_at = NULL",
        params![relationship_id, user_id.cast_signed(), now_ts()],
    )?;

    Ok(())
}

fn maybe_end_relationship(conn: &Connection, relationship_id: i64) -> Result<bool> {
    let active_members: i64 = conn.query_row(
        "SELECT COUNT(*) FROM relationship_members WHERE relationship_id = ? AND left_at IS NULL",
        [relationship_id],
        |row| row.get(0),
    )?;

    if active_members < 2 {
        let updated = conn.execute(
            "UPDATE relationships
             SET status = 'ended', ended_at = ?
             WHERE id = ? AND status = 'active'",
            params![now_ts(), relationship_id],
        )?;
        return Ok(updated > 0);
    }

    Ok(false)
}

fn shared_relationships_by_type(
    conn: &Connection,
    guild_id: u64,
    relationship_type: &str,
    user_a: u64,
    user_b: u64,
) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare(
        "SELECT r.id
         FROM relationships r
         WHERE r.guild_id = ?
           AND r.relationship_type = ?
           AND r.status = 'active'
           AND EXISTS (
                SELECT 1
                FROM relationship_members m1
                WHERE m1.relationship_id = r.id
                  AND m1.user_id = ?
                  AND m1.left_at IS NULL
           )
           AND EXISTS (
                SELECT 1
                FROM relationship_members m2
                WHERE m2.relationship_id = r.id
                  AND m2.user_id = ?
                  AND m2.left_at IS NULL
           )
         ORDER BY r.id ASC",
    )?;

    let rows = stmt.query_map(
        params![
            guild_id.cast_signed(),
            relationship_type,
            user_a.cast_signed(),
            user_b.cast_signed(),
        ],
        |row| row.get(0),
    )?;

    let mut ids = Vec::new();
    for row in rows {
        ids.push(row?);
    }

    Ok(ids)
}

fn caller_active_relationships_by_type(
    conn: &Connection,
    guild_id: u64,
    relationship_type: &str,
    caller_id: u64,
) -> Result<Vec<i64>> {
    let mut stmt = conn.prepare(
        "SELECT r.id
         FROM relationships r
         INNER JOIN relationship_members m ON m.relationship_id = r.id
         WHERE r.guild_id = ?
           AND r.relationship_type = ?
           AND r.status = 'active'
           AND m.user_id = ?
           AND m.left_at IS NULL
         ORDER BY r.id ASC",
    )?;

    let rows = stmt.query_map(
        params![
            guild_id.cast_signed(),
            relationship_type,
            caller_id.cast_signed(),
        ],
        |row| row.get(0),
    )?;

    let mut ids = Vec::new();
    for row in rows {
        ids.push(row?);
    }

    Ok(ids)
}
