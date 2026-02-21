mod db;
mod logic;
mod reply;

use crate::types::Context;
use color_eyre::eyre::Result;
use poise::serenity_prelude::User;
use std::collections::BTreeMap;

use db::RELATIONSHIP_DB;
use logic::{
    accept_invite, active_member_ids, decline_invite, has_pending_invite, is_active_member,
    leave_relationship, list_active_relationship_groups, list_active_relationships_for_user,
    list_pending_invites_for_user, normalize_relationship_type, resolve_relationship_for_make,
    shared_relationship_ids, try_create_invite,
};
use reply::safe_reply;

fn trim_optional_text(input: Option<String>) -> Option<String> {
    input.and_then(|value| {
        let trimmed = value.trim().to_string();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed)
        }
    })
}

fn format_group_header(
    relationship_type: &str,
    emoji: Option<&str>,
    description: Option<&str>,
) -> String {
    let mut base = String::new();
    if let Some(emoji) = emoji {
        base.push_str(emoji);
        base.push(' ');
    }
    base.push('`');
    base.push_str(relationship_type);
    base.push('`');
    if let Some(description) = description {
        base.push_str(" - ");
        base.push_str(description);
    }
    base
}

#[poise::command(
    slash_command,
    guild_only,
    subcommands(
        "help", "make", "accept", "decline", "end", "leave", "list", "inbox", "groups"
    )
)]
pub async fn relationship(_: Context<'_>) -> Result<()> {
    Ok(())
}

/// Show how relationship commands work.
#[poise::command(slash_command, guild_only)]
pub async fn help(ctx: Context<'_>) -> Result<()> {
    let text = [
        "relationshipdb guide :3",
        "",
        "**How work**",
        "- Relationship groups are identified by an ID (like `#12`).",
        "- Types are free-form like `marriage`, `friend`, `adopted-sibling`.",
        "- Groups can have optional emoji + description metadata.",
        "- `make` sends an invite. The other user must `accept`.",
        "",
        "**How to use**",
        "1. `/relationship make relationship_type:marriage user:@kitten`",
        "2. Check invites with `/relationship inbox`",
        "3. Accept with `/relationship accept relationship_id:<id>`",
        "",
        "**Commands**",
        "- `/relationship make <type> <user> [relationship_id] [emoji] [description]`: create/invite.",
        "- `/relationship inbox`: show your pending invites.",
        "- `/relationship accept <relationship_id>`: join invited group.",
        "- `/relationship decline <relationship_id>`: decline invite.",
        "- `/relationship list [user]`: show active groups for a user.",
        "- `/relationship groups`: show all active groups in this server.",
        "- `/relationship end <type> <user>`: leave a shared group by type.",
        "- `/relationship leave <relationship_id>`: leave a group by ID.",
        "",
        "**Notes**",
        "- If `end` is ambiguous (multiple matches), use `leave` with an ID.",
        "- If a group drops below 2 active members, it auto-ends.",
    ]
    .join("\n");

    ctx.send(safe_reply().content(text).ephemeral(true)).await?;
    Ok(())
}

/// Create a relationship invite for another user.
#[poise::command(slash_command, guild_only)]
pub async fn make(
    ctx: Context<'_>,
    #[description = "Relationship type (e.g. marriage, friend, adopted-sibling)"]
    relationship_type: String,
    #[description = "User to invite"] user: User,
    #[description = "Existing relationship ID to invite into (optional)"] relationship_id: Option<
        i64,
    >,
    #[description = "Optional emoji for new group (e.g. 💍)"] emoji: Option<String>,
    #[description = "Optional description for new group"] description: Option<String>,
) -> Result<()> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.send(
            safe_reply()
                .content("This command can only be used in a server.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let caller_id = ctx.author().id.get();
    let target_id = user.id.get();

    if target_id == caller_id {
        ctx.send(
            safe_reply()
                .content("You cannot create a relationship with yourself.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    if user.bot {
        ctx.send(
            safe_reply()
                .content("You cannot create relationships with bot accounts.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let normalized_type = match normalize_relationship_type(&relationship_type) {
        Ok(value) => value,
        Err(err) => {
            ctx.send(safe_reply().content(err.to_string()).ephemeral(true))
                .await?;
            return Ok(());
        }
    };

    let emoji = trim_optional_text(emoji);
    let description = trim_optional_text(description);

    let relationship_result = {
        let conn = RELATIONSHIP_DB.lock().unwrap();
        resolve_relationship_for_make(
            &conn,
            guild_id.get(),
            caller_id,
            &normalized_type,
            relationship_id,
            emoji.as_deref(),
            description.as_deref(),
        )?
    };

    let make_resolution = match relationship_result {
        Ok(resolution) => resolution,
        Err(message) => {
            ctx.send(safe_reply().content(message).ephemeral(true))
                .await?;
            return Ok(());
        }
    };

    let invite_error = {
        let conn = RELATIONSHIP_DB.lock().unwrap();
        try_create_invite(&conn, make_resolution.relationship_id, caller_id, target_id)?
    };

    if let Some(message) = invite_error {
        ctx.send(safe_reply().content(message).ephemeral(true))
            .await?;
        return Ok(());
    }

    let preface = if make_resolution.created_new_group {
        "Created a new relationship group and sent invite"
    } else {
        "Sent invite"
    };

    let relationship_id = make_resolution.relationship_id;
    ctx.send(
        safe_reply()
            .content(format!(
                "{preface} for `{normalized_type}` to <@{target_id}> in relationship #{relationship_id}.\nThey can run `/relationship accept relationship_id:{relationship_id}` or `/relationship decline relationship_id:{relationship_id}`."
            ))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// Accept a pending relationship invite.
#[poise::command(slash_command, guild_only)]
pub async fn accept(
    ctx: Context<'_>,
    #[description = "Relationship ID to accept"] relationship_id: i64,
) -> Result<()> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.send(
            safe_reply()
                .content("This command can only be used in a server.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let caller_id = ctx.author().id.get();

    let has_pending = {
        let conn = RELATIONSHIP_DB.lock().unwrap();
        has_pending_invite(&conn, relationship_id, caller_id, guild_id.get())
    };

    if !has_pending {
        ctx.send(
            safe_reply()
                .content("No pending invite found for that relationship ID.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    {
        let conn = RELATIONSHIP_DB.lock().unwrap();
        accept_invite(&conn, relationship_id, caller_id)?;
    }

    ctx.send(
        safe_reply()
            .content(format!(
                "Accepted invite for relationship #{relationship_id}."
            ))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// Decline a pending relationship invite.
#[poise::command(slash_command, guild_only)]
pub async fn decline(
    ctx: Context<'_>,
    #[description = "Relationship ID to decline"] relationship_id: i64,
) -> Result<()> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.send(
            safe_reply()
                .content("This command can only be used in a server.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let caller_id = ctx.author().id.get();

    let updated = {
        let conn = RELATIONSHIP_DB.lock().unwrap();
        decline_invite(&conn, relationship_id, caller_id, guild_id.get())?
    };

    if updated == 0 {
        ctx.send(
            safe_reply()
                .content("No pending invite found for that relationship ID.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    ctx.send(
        safe_reply()
            .content(format!(
                "Declined invite for relationship #{relationship_id}."
            ))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// Leave a shared relationship by type with another user.
#[poise::command(slash_command, guild_only)]
pub async fn end(
    ctx: Context<'_>,
    #[description = "Relationship type (e.g. marriage, friend, adopted-sibling)"]
    relationship_type: String,
    #[description = "A user in the relationship"] user: User,
) -> Result<()> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.send(
            safe_reply()
                .content("This command can only be used in a server.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let caller_id = ctx.author().id.get();
    let target_id = user.id.get();

    let normalized_type = match normalize_relationship_type(&relationship_type) {
        Ok(value) => value,
        Err(err) => {
            ctx.send(safe_reply().content(err.to_string()).ephemeral(true))
                .await?;
            return Ok(());
        }
    };

    let shared_ids = {
        let conn = RELATIONSHIP_DB.lock().unwrap();
        shared_relationship_ids(
            &conn,
            guild_id.get(),
            &normalized_type,
            caller_id,
            target_id,
        )?
    };

    if shared_ids.is_empty() {
        ctx.send(
            safe_reply()
                .content(format!(
                    "No active `{normalized_type}` relationship found that includes both of you."
                ))
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    if shared_ids.len() > 1 {
        let ids = shared_ids
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ");

        ctx.send(
            safe_reply()
                .content(format!(
                    "Multiple matching relationships found: {ids}. Use `/relationship leave relationship_id:<id>` to choose exactly one."
                ))
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let relationship_id = shared_ids[0];
    let relationship_ended = {
        let conn = RELATIONSHIP_DB.lock().unwrap();
        leave_relationship(&conn, relationship_id, caller_id)?
    };

    let suffix = if relationship_ended {
        " The relationship was automatically ended because fewer than 2 active members remain."
    } else {
        ""
    };

    ctx.send(
        safe_reply()
            .content(format!(
                "You left `{normalized_type}` relationship #{relationship_id}.{suffix}"
            ))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// Leave a relationship by ID.
#[poise::command(slash_command, guild_only)]
pub async fn leave(
    ctx: Context<'_>,
    #[description = "Relationship ID to leave"] relationship_id: i64,
) -> Result<()> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.send(
            safe_reply()
                .content("This command can only be used in a server.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let caller_id = ctx.author().id.get();

    let is_member = {
        let conn = RELATIONSHIP_DB.lock().unwrap();
        is_active_member(&conn, relationship_id, guild_id.get(), caller_id)
    };

    if !is_member {
        ctx.send(
            safe_reply()
                .content("You are not an active member of that relationship ID.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let relationship_ended = {
        let conn = RELATIONSHIP_DB.lock().unwrap();
        leave_relationship(&conn, relationship_id, caller_id)?
    };

    let suffix = if relationship_ended {
        " The relationship was automatically ended because fewer than 2 active members remain."
    } else {
        ""
    };

    ctx.send(
        safe_reply()
            .content(format!("You left relationship #{relationship_id}.{suffix}"))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// List active relationships for yourself or another user.
#[poise::command(slash_command, guild_only)]
pub async fn list(
    ctx: Context<'_>,
    #[description = "User to inspect (defaults to you)"] user: Option<User>,
) -> Result<()> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.send(
            safe_reply()
                .content("This command can only be used in a server.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let target = user.as_ref().unwrap_or(ctx.author());
    let target_id = target.id.get();

    let relationships = {
        let conn = RELATIONSHIP_DB.lock().unwrap();
        list_active_relationships_for_user(&conn, guild_id.get(), target_id)?
    };

    if relationships.is_empty() {
        ctx.send(safe_reply().content(format!("<@{target_id}> has no active relationships.")))
            .await?;
        return Ok(());
    }

    let mut grouped: BTreeMap<String, Vec<(i64, Option<String>, Option<String>)>> = BTreeMap::new();
    for (relationship_id, relationship_type, emoji, description) in &relationships {
        grouped.entry(relationship_type.clone()).or_default().push((
            *relationship_id,
            emoji.clone(),
            description.clone(),
        ));
    }

    let mut lines = vec![format!("**Active relationships for <@{target_id}>**")];

    {
        let conn = RELATIONSHIP_DB.lock().unwrap();
        for (relationship_type, rows) in grouped {
            lines.push(format!("\n`{relationship_type}`"));
            for (relationship_id, emoji, description) in rows {
                let members = active_member_ids(&conn, relationship_id)?;
                let member_mentions = members
                    .iter()
                    .map(|id| format!("<@{id}>"))
                    .collect::<Vec<_>>()
                    .join(", ");

                let header = format_group_header(
                    &relationship_type,
                    emoji.as_deref(),
                    description.as_deref(),
                );
                lines.push(format!("- #{relationship_id} {header}: {member_mentions}"));
            }
        }
    }

    ctx.send(safe_reply().content(lines.join("\n"))).await?;
    Ok(())
}

/// Show your pending relationship invites.
#[poise::command(slash_command, guild_only)]
pub async fn inbox(ctx: Context<'_>) -> Result<()> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.send(
            safe_reply()
                .content("This command can only be used in a server.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let caller_id = ctx.author().id.get();
    let invites = {
        let conn = RELATIONSHIP_DB.lock().unwrap();
        list_pending_invites_for_user(&conn, guild_id.get(), caller_id)?
    };

    if invites.is_empty() {
        ctx.send(
            safe_reply()
                .content("You have no pending relationship invites.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    let mut lines = vec!["**Your pending relationship invites**".to_string()];
    for invite in invites {
        let header = format_group_header(
            &invite.relationship_type,
            invite.emoji.as_deref(),
            invite.description.as_deref(),
        );
        lines.push(format!(
            "- `#{}` {} from <@{}> (created <t:{}:R>)\n  Accept: `/relationship accept relationship_id:{}` | Decline: `/relationship decline relationship_id:{}`",
            invite.relationship_id,
            header,
            invite.inviter_id,
            invite.created_at,
            invite.relationship_id,
            invite.relationship_id
        ));
    }

    ctx.send(safe_reply().content(lines.join("\n")).ephemeral(true))
        .await?;
    Ok(())
}

/// Show all active relationship groups in this server.
#[poise::command(slash_command, guild_only)]
pub async fn groups(ctx: Context<'_>) -> Result<()> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.send(
            safe_reply()
                .content("This command can only be used in a server.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let groups = {
        let conn = RELATIONSHIP_DB.lock().unwrap();
        list_active_relationship_groups(&conn, guild_id.get())?
    };

    if groups.is_empty() {
        ctx.send(safe_reply().content("No active relationship groups exist in this server."))
            .await?;
        return Ok(());
    }

    let mut lines = vec!["**Active relationship groups in this server**".to_string()];
    for group in groups {
        let members = group
            .member_ids
            .iter()
            .map(|id| format!("<@{id}>"))
            .collect::<Vec<_>>()
            .join(", ");

        let header = format_group_header(
            &group.relationship_type,
            group.emoji.as_deref(),
            group.description.as_deref(),
        );
        lines.push(format!(
            "- `#{}` {}: {}",
            group.relationship_id, header, members
        ));
    }

    ctx.send(safe_reply().content(lines.join("\n"))).await?;
    Ok(())
}
