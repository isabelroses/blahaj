use base64::Engine;
use color_eyre::eyre::{Result, eyre};
use poise::CreateReply;
use poise::serenity_prelude::{Emoji, EmojiId, GuildId, Http, User, UserId};
use reqwest::Client;
use rusqlite::params;

use crate::types::Context;
use crate::utils::DB;

pub(crate) fn sanitize_emoji_name(name: &str) -> String {
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

#[derive(Debug)]
pub struct AvatarEmoji {
    pub guild_id: GuildId,
    pub emoji_id: EmojiId,
    pub emoji_name: String,
    pub avatar_url: Option<String>,
}

fn get_avatar_emoji(user_id: UserId, guild_id: GuildId) -> Option<AvatarEmoji> {
    let conn = DB.lock().ok()?;
    let mut stmt = conn
        .prepare(
            "SELECT emoji_id, emoji_name, avatar_url FROM avatar_emojis WHERE user_id = ? AND guild_id = ?",
        )
        .ok()?;

    stmt.query_row(
        params![user_id.get().cast_signed(), guild_id.get().cast_signed()],
        |row| {
            let emoji_id: i64 = row.get(0)?;
            let emoji_name: String = row.get(1)?;
            let avatar_url: Option<String> = row.get(2)?;
            Ok(AvatarEmoji {
                guild_id,
                emoji_id: EmojiId::new(emoji_id.cast_unsigned()),
                emoji_name,
                avatar_url,
            })
        },
    )
    .ok()
}

pub fn get_avatar_emojis_for_user(user_id: UserId) -> Result<Vec<AvatarEmoji>> {
    let conn = DB.lock().unwrap();
    let mut stmt = conn.prepare(
        "SELECT guild_id, emoji_id, emoji_name, avatar_url FROM avatar_emojis WHERE user_id = ?",
    )?;
    let rows = stmt.query_map(params![user_id.get().cast_signed()], |row| {
        let guild_id: i64 = row.get(0)?;
        let emoji_id: i64 = row.get(1)?;
        let emoji_name: String = row.get(2)?;
        let avatar_url: Option<String> = row.get(3)?;
        Ok(AvatarEmoji {
            guild_id: GuildId::new(guild_id.cast_unsigned()),
            emoji_id: EmojiId::new(emoji_id.cast_unsigned()),
            emoji_name,
            avatar_url,
        })
    })?;

    rows.collect::<rusqlite::Result<Vec<_>>>()
        .map_err(Into::into)
}

fn save_avatar_emoji(
    user_id: UserId,
    guild_id: GuildId,
    emoji_id: EmojiId,
    emoji_name: &str,
    avatar_url: &str,
) -> Result<()> {
    let conn = DB.lock().unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO avatar_emojis (user_id, guild_id, emoji_id, emoji_name, avatar_url) VALUES (?, ?, ?, ?, ?)",
        params![
            user_id.get().cast_signed(),
            guild_id.get().cast_signed(),
            emoji_id.get().cast_signed(),
            emoji_name,
            avatar_url,
        ],
    )?;
    Ok(())
}

fn delete_avatar_emoji(user_id: UserId, guild_id: GuildId) -> Result<()> {
    let conn = DB.lock().unwrap();
    conn.execute(
        "DELETE FROM avatar_emojis WHERE user_id = ? AND guild_id = ?",
        params![user_id.get().cast_signed(), guild_id.get().cast_signed()],
    )?;
    Ok(())
}

async fn fetch_avatar_data_uri(client: &Client, avatar_url: &str) -> Result<String> {
    // Request a reasonably sized image (128x128 is good for emoji)
    let base_url = avatar_url
        .split_once('?')
        .map_or(avatar_url, |(url, _)| url);
    let url = format!("{base_url}?size=128");

    let response = client
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

pub async fn refresh_avatar_emoji(
    http: &Http,
    client: &Client,
    user_id: UserId,
    guild_id: GuildId,
    emoji_name: &str,
    avatar_url: &str,
    force: bool,
) -> Result<Option<Emoji>> {
    let Some(existing) = get_avatar_emoji(user_id, guild_id) else {
        return Ok(None);
    };

    if !force && existing.avatar_url.as_deref() == Some(avatar_url) {
        return Ok(None);
    }

    // Discord doesn't support editing emoji images, so delete and recreate.
    guild_id
        .delete_emoji(http, existing.emoji_id)
        .await
        .map_err(|e| eyre!("Failed to delete old emoji: {e}"))?;

    let data_uri = fetch_avatar_data_uri(client, avatar_url).await?;
    let emoji = guild_id
        .create_emoji(http, emoji_name, &data_uri)
        .await
        .map_err(|e| eyre!("Failed to create new emoji: {e}"))?;

    save_avatar_emoji(user_id, guild_id, emoji.id, emoji_name, avatar_url)?;
    Ok(Some(emoji))
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

    let user_id = user.id;

    if let Some(existing) = get_avatar_emoji(user_id, guild_id) {
        ctx.send(
            CreateReply::default()
                .content(format!(
                    "An avatar emoji already exists for <@{user_id}> as `:{}:`. Use `/avatarsync update` to refresh it.",
                    existing.emoji_name
                ))
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    ctx.defer().await?;

    let avatar_url = user.face();
    let data_uri = fetch_avatar_data_uri(&ctx.data().client, &avatar_url).await?;
    let emoji_name = sanitize_emoji_name(&user.name);

    let emoji = guild_id
        .create_emoji(ctx.http(), &emoji_name, &data_uri)
        .await
        .map_err(|e| eyre!("Failed to create emoji: {e}"))?;

    save_avatar_emoji(user_id, guild_id, emoji.id, &emoji_name, &avatar_url)?;

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

    let user_id = user.id;

    if get_avatar_emoji(user_id, guild_id).is_none() {
        ctx.send(
            CreateReply::default()
                .content(format!(
                    "No avatar emoji found for <@{user_id}>. Use `/avatarsync create` first."
                ))
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    }

    ctx.defer().await?;

    let avatar_url = user.face();
    let emoji_name = sanitize_emoji_name(&user.name);
    let emoji = refresh_avatar_emoji(
        ctx.http(),
        &ctx.data().client,
        user_id,
        guild_id,
        &emoji_name,
        &avatar_url,
        true,
    )
    .await?
    .ok_or_else(|| eyre!("No avatar emoji found for <@{user_id}>."))?;

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

    let user_id = user.id;

    let Some(existing) = get_avatar_emoji(user_id, guild_id) else {
        ctx.send(
            CreateReply::default()
                .content(format!("No avatar emoji found for <@{user_id}>."))
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    guild_id
        .delete_emoji(ctx.http(), existing.emoji_id)
        .await
        .map_err(|e| eyre!("Failed to delete emoji: {e}"))?;

    delete_avatar_emoji(user_id, guild_id)?;

    ctx.say(format!(
        "Deleted avatar emoji `:{}:` for <@{user_id}>.",
        existing.emoji_name
    ))
    .await?;

    Ok(())
}
