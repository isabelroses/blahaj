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
    leave_relationship, list_active_relationships_for_user, normalize_relationship_type,
    resolve_relationship_for_make, shared_relationship_ids, try_create_invite,
};
use reply::safe_reply;

#[poise::command(
    slash_command,
    guild_only,
    subcommands("make", "accept", "decline", "end", "leave", "list")
)]
pub async fn relationship(_: Context<'_>) -> Result<()> {
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

    let relationship_result = {
        let conn = RELATIONSHIP_DB.lock().unwrap();
        resolve_relationship_for_make(
            &conn,
            guild_id.get(),
            caller_id,
            &normalized_type,
            relationship_id,
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

    let mut grouped: BTreeMap<String, Vec<i64>> = BTreeMap::new();
    for (relationship_id, relationship_type) in &relationships {
        grouped
            .entry(relationship_type.clone())
            .or_default()
            .push(*relationship_id);
    }

    let mut lines = vec![format!("**Active relationships for <@{target_id}>**")];

    {
        let conn = RELATIONSHIP_DB.lock().unwrap();
        for (relationship_type, ids) in grouped {
            lines.push(format!("\n`{relationship_type}`"));
            for relationship_id in ids {
                let members = active_member_ids(&conn, relationship_id)?;
                let member_mentions = members
                    .iter()
                    .map(|id| format!("<@{id}>"))
                    .collect::<Vec<_>>()
                    .join(", ");

                lines.push(format!("- #{relationship_id}: {member_mentions}"));
            }
        }
    }

    ctx.send(safe_reply().content(lines.join("\n"))).await?;
    Ok(())
}
