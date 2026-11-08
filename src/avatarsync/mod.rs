//! Guild emojis made from users' avatars, kept in sync when an avatar changes.
//!
//! `/avatarsync` and the avatar-change events are both thin adapters over
//! [`create`], [`refresh`] and [`delete`].

pub mod store;

use base64::Engine;
use color_eyre::eyre::{Result, eyre};
use poise::serenity_prelude::{Emoji, EmojiId, GuildId, Http, User};
use reqwest::Client;

use crate::db;
use store::AvatarEmoji;

pub enum Created {
    Created(Box<Emoji>),
    /// The user already has a synced emoji in this guild, with this name.
    AlreadyExists(String),
}

/// The name of `user`'s synced emoji in `guild_id`, if they have one.
pub fn synced_name(user: &User, guild_id: GuildId) -> Result<Option<String>> {
    let existing = db::with(|conn| store::get(conn, user.id.get(), guild_id.get()))?;
    Ok(existing.map(|e| e.emoji_name))
}

/// Makes a guild emoji from `user`'s avatar, named after their username.
pub async fn create(
    http: &Http,
    client: &Client,
    user: &User,
    guild_id: GuildId,
) -> Result<Created> {
    if let Some(existing) = db::with(|conn| store::get(conn, user.id.get(), guild_id.get()))? {
        return Ok(Created::AlreadyExists(existing.emoji_name));
    }

    let avatar_url = user.face();
    let data_uri = fetch_avatar_data_uri(client, &avatar_url).await?;
    let name = sanitize_emoji_name(&user.name);
    let emoji = guild_id
        .create_emoji(http, &name, &data_uri)
        .await
        .map_err(|e| eyre!("Failed to create emoji: {e}"))?;

    let row = AvatarEmoji {
        guild_id: guild_id.get(),
        emoji_id: emoji.id.get(),
        emoji_name: name,
        avatar_url: Some(avatar_url),
    };
    db::with(|conn| store::save(conn, user.id.get(), &row))?;
    Ok(Created::Created(Box::new(emoji)))
}

/// Replaces `user`'s synced emoji in `guild_id` with their current avatar,
/// keeping its name. Returns `None` if they have no synced emoji there, or if
/// the avatar hasn't changed and `force` is false.
pub async fn refresh(
    http: &Http,
    client: &Client,
    user: &User,
    guild_id: GuildId,
    force: bool,
) -> Result<Option<Emoji>> {
    let Some(existing) = db::with(|conn| store::get(conn, user.id.get(), guild_id.get()))? else {
        return Ok(None);
    };

    let avatar_url = user.face();
    if !force && existing.avatar_url.as_deref() == Some(avatar_url.as_str()) {
        return Ok(None);
    }

    // Discord can't change an emoji's image, so make the new emoji first and
    // only then remove the old one. If anything fails before that, the old
    // emoji and its row are left as they were.
    let data_uri = fetch_avatar_data_uri(client, &avatar_url).await?;
    let emoji = guild_id
        .create_emoji(http, &existing.emoji_name, &data_uri)
        .await
        .map_err(|e| eyre!("Failed to create new emoji: {e}"))?;

    let row = AvatarEmoji {
        emoji_id: emoji.id.get(),
        avatar_url: Some(avatar_url),
        ..existing.clone()
    };
    db::with(|conn| store::save(conn, user.id.get(), &row))?;

    if let Err(e) = guild_id
        .delete_emoji(http, EmojiId::new(existing.emoji_id))
        .await
    {
        eprintln!(
            "avatarsync: made the new emoji for {} but couldn't delete the old one ({}): {e}",
            user.id, existing.emoji_id
        );
    }

    Ok(Some(emoji))
}

/// Refreshes `user`'s synced emoji in every guild that has one.
pub async fn refresh_everywhere(http: &Http, client: &Client, user: &User) -> Result<()> {
    let synced = db::with(|conn| store::for_user(conn, user.id.get()))?;
    for emoji in synced {
        let guild_id = GuildId::new(emoji.guild_id);
        if let Err(e) = refresh(http, client, user, guild_id, false).await {
            eprintln!(
                "avatarsync: failed to refresh {}'s emoji in {guild_id}: {e}",
                user.id
            );
        }
    }
    Ok(())
}

/// Deletes `user`'s synced emoji in `guild_id`, returning its name, or `None` if
/// there wasn't one.
pub async fn delete(http: &Http, user: &User, guild_id: GuildId) -> Result<Option<String>> {
    let Some(existing) = db::with(|conn| store::get(conn, user.id.get(), guild_id.get()))? else {
        return Ok(None);
    };

    guild_id
        .delete_emoji(http, EmojiId::new(existing.emoji_id))
        .await
        .map_err(|e| eyre!("Failed to delete emoji: {e}"))?;
    db::with(|conn| store::delete(conn, user.id.get(), guild_id.get()))?;

    Ok(Some(existing.emoji_name))
}

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
        .and_then(reqwest::Response::error_for_status)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emoji_names_are_discord_safe() {
        assert_eq!(sanitize_emoji_name("isabel.roses"), "isabel_roses");
        assert_eq!(sanitize_emoji_name("a"), "a_avatar");
    }
}
