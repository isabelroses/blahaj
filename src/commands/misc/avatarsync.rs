use base64::Engine;
use color_eyre::eyre::{Result, eyre};
use poise::CreateReply;
use poise::serenity_prelude::{EmojiId, User};
use rusqlite::params;

use crate::types::Context;
use crate::utils::DB;

fn sanitize_emoji_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.len() < 2 {
        format!("{sanitized}_avatar")
    } else {
        sanitized
    }
}

fn get_avatar_emoji(user_id: u64, guild_id: u64) -> Option<(EmojiId, String)> {
    let conn = DB.lock().ok()?;
    let mut stmt = conn
        .prepare(
            "SELECT emoji_id, emoji_name FROM avatar_emojis WHERE user_id = ? AND guild_id = ?",
        )
        .ok()?;

    stmt.query_row(
        params![user_id.cast_signed(), guild_id.cast_signed()],
        |row| {
            let emoji_id: i64 = row.get(0)?;
            let emoji_name: String = row.get(1)?;
            Ok((EmojiId::new(emoji_id.cast_unsigned()), emoji_name))
        },
    )
    .ok()
}

fn save_avatar_emoji(
    user_id: u64,
    guild_id: u64,
    emoji_id: EmojiId,
    emoji_name: &str,
) -> Result<()> {
    let conn = DB.lock().unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO avatar_emojis (user_id, guild_id, emoji_id, emoji_name) VALUES (?, ?, ?, ?)",
        params![user_id.cast_signed(), guild_id.cast_signed(), emoji_id.get().cast_signed(), emoji_name],
    )?;
    Ok(())
}

fn delete_avatar_emoji(user_id: u64, guild_id: u64) -> Result<()> {
    let conn = DB.lock().unwrap();
    conn.execute(
        "DELETE FROM avatar_emojis WHERE user_id = ? AND guild_id = ?",
        params![user_id.cast_signed(), guild_id.cast_signed()],
    )?;
    Ok(())
}

async fn fetch_avatar_data_uri(ctx: Context<'_>, user: &User) -> Result<String> {
    let avatar_url = user.face();
    // Request a reasonably sized image (128x128 is good for emoji)
    let url = format!("{avatar_url}?size=128");

    let response = ctx
        .data()
        .client
        .get(&url)
        .send()
        .await
        .map_err(|e| eyre!("Failed to download avatar: {e}"))?;

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/png")
        .to_string();

    let mimetype = if content_type.contains("gif") {
        "image/gif"
    } else if content_type.contains("webp") {
        "image/webp"
    } else if content_type.contains("jpeg") || content_type.contains("jpg") {
        "image/jpeg"
    } else {
        "image/png"
    };

    let bytes = response
        .bytes()
        .await
        .map_err(|e| eyre!("Failed to read avatar bytes: {e}"))?;

    let encoded = base64::prelude::BASE64_STANDARD.encode(&bytes);
    Ok(format!("data:{mimetype};base64,{encoded}"))
}

/// Manage guild emojis synced from user avatars.
#[allow(clippy::unused_async)]
#[poise::command(
    slash_command,
    guild_only,
    required_permissions = "ADMINISTRATOR",
    subcommands("create", "update", "delete")
)]
pub async fn avatarsync(_: Context<'_>) -> Result<()> {
    Ok(())
}

/// Create a guild emoji from a user's avatar.
#[poise::command(slash_command, guild_only, required_permissions = "ADMINISTRATOR")]
pub async fn create(
    ctx: Context<'_>,
    #[description = "User whose avatar to create an emoji from"] user: User,
) -> Result<()> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.send(
            CreateReply::default()
                .content("This command can only be used in a server.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let user_id = user.id.get();
    let guild_id_val = guild_id.get();

    if let Some((_, existing_name)) = get_avatar_emoji(user_id, guild_id_val) {
        ctx.send(
            CreateReply::default()
                .content(format!(
                    "An avatar emoji already exists for <@{user_id}> as `:{existing_name}:`. Use `/avatarsync update` to refresh it."
                ))
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    ctx.defer().await?;

    let data_uri = fetch_avatar_data_uri(ctx, &user).await?;
    let emoji_name = sanitize_emoji_name(&user.name);

    let emoji = guild_id
        .create_emoji(ctx.http(), &emoji_name, &data_uri)
        .await
        .map_err(|e| eyre!("Failed to create emoji: {e}"))?;

    save_avatar_emoji(user_id, guild_id_val, emoji.id, &emoji_name)?;

    ctx.say(format!(
        "Created avatar emoji <:{}:{}> for <@{user_id}>.",
        emoji.name, emoji.id
    ))
    .await?;

    Ok(())
}

/// Update an existing avatar emoji with the user's current avatar.
#[poise::command(slash_command, guild_only, required_permissions = "ADMINISTRATOR")]
pub async fn update(
    ctx: Context<'_>,
    #[description = "User whose avatar emoji to update"] user: User,
) -> Result<()> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.send(
            CreateReply::default()
                .content("This command can only be used in a server.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let user_id = user.id.get();
    let guild_id_val = guild_id.get();

    let Some((old_emoji_id, _)) = get_avatar_emoji(user_id, guild_id_val) else {
        ctx.send(
            CreateReply::default()
                .content(format!(
                    "No avatar emoji found for <@{user_id}>. Use `/avatarsync create` first."
                ))
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    ctx.defer().await?;

    // Discord doesn't support editing emoji images, so delete and recreate
    guild_id
        .delete_emoji(ctx.http(), old_emoji_id)
        .await
        .map_err(|e| eyre!("Failed to delete old emoji: {e}"))?;

    let data_uri = fetch_avatar_data_uri(ctx, &user).await?;
    let emoji_name = sanitize_emoji_name(&user.name);

    let emoji = guild_id
        .create_emoji(ctx.http(), &emoji_name, &data_uri)
        .await
        .map_err(|e| eyre!("Failed to create new emoji: {e}"))?;

    save_avatar_emoji(user_id, guild_id_val, emoji.id, &emoji_name)?;

    ctx.say(format!(
        "Updated avatar emoji <:{}:{}> for <@{user_id}>.",
        emoji.name, emoji.id
    ))
    .await?;

    Ok(())
}

/// Delete an avatar emoji for a user.
#[poise::command(slash_command, guild_only, required_permissions = "ADMINISTRATOR")]
pub async fn delete(
    ctx: Context<'_>,
    #[description = "User whose avatar emoji to delete"] user: User,
) -> Result<()> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.send(
            CreateReply::default()
                .content("This command can only be used in a server.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let user_id = user.id.get();
    let guild_id_val = guild_id.get();

    let Some((emoji_id, emoji_name)) = get_avatar_emoji(user_id, guild_id_val) else {
        ctx.send(
            CreateReply::default()
                .content(format!("No avatar emoji found for <@{user_id}>."))
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    guild_id
        .delete_emoji(ctx.http(), emoji_id)
        .await
        .map_err(|e| eyre!("Failed to delete emoji: {e}"))?;

    delete_avatar_emoji(user_id, guild_id_val)?;

    ctx.say(format!(
        "Deleted avatar emoji `:{emoji_name}:` for <@{user_id}>."
    ))
    .await?;

    Ok(())
}
