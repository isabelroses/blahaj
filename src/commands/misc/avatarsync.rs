use color_eyre::eyre::Result;
use poise::CreateReply;
use poise::serenity_prelude::{GuildId, User};

use crate::avatarsync::{self, Created};
use crate::types::Context;

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

/// The guild the command ran in. Every subcommand is `guild_only`, so this is
/// always set.
fn guild(ctx: Context<'_>) -> GuildId {
    ctx.guild_id().expect("avatarsync commands are guild_only")
}

async fn say_ephemeral(ctx: Context<'_>, text: String) -> Result<()> {
    ctx.send(CreateReply::default().content(text).ephemeral(true))
        .await?;
    Ok(())
}

fn already_exists(user: &User, name: &str) -> String {
    format!(
        "An avatar emoji already exists for <@{}> as `:{name}:`. Use `/avatarsync update` to refresh it.",
        user.id
    )
}

fn not_found_for_update(user: &User) -> String {
    format!(
        "No avatar emoji found for <@{}>. Use `/avatarsync create` first.",
        user.id
    )
}

/// Create a guild emoji from a user's avatar.
#[poise::command(slash_command, guild_only, required_permissions = "ADMINISTRATOR")]
pub async fn create(
    ctx: Context<'_>,
    #[description = "User whose avatar to create an emoji from"] user: User,
) -> Result<()> {
    // Check before deferring so the "already exists" reply can be ephemeral.
    if let Some(name) = avatarsync::synced_name(&user, guild(ctx))? {
        return say_ephemeral(ctx, already_exists(&user, &name)).await;
    }
    ctx.defer().await?;

    match avatarsync::create(ctx.http(), &ctx.data().client, &user, guild(ctx)).await? {
        Created::Created(emoji) => {
            ctx.say(format!(
                "Created avatar emoji <:{}:{}> for <@{}>.",
                emoji.name, emoji.id, user.id
            ))
            .await?;
        }
        Created::AlreadyExists(name) => {
            ctx.say(already_exists(&user, &name)).await?;
        }
    }
    Ok(())
}

/// Update an existing avatar emoji with the user's current avatar.
#[poise::command(slash_command, guild_only, required_permissions = "ADMINISTRATOR")]
pub async fn update(
    ctx: Context<'_>,
    #[description = "User whose avatar emoji to update"] user: User,
) -> Result<()> {
    if avatarsync::synced_name(&user, guild(ctx))?.is_none() {
        return say_ephemeral(ctx, not_found_for_update(&user)).await;
    }
    ctx.defer().await?;

    match avatarsync::refresh(ctx.http(), &ctx.data().client, &user, guild(ctx), true).await? {
        Some(emoji) => {
            ctx.say(format!(
                "Updated avatar emoji <:{}:{}> for <@{}>.",
                emoji.name, emoji.id, user.id
            ))
            .await?;
        }
        None => {
            ctx.say(not_found_for_update(&user)).await?;
        }
    }
    Ok(())
}

/// Delete an avatar emoji for a user.
#[poise::command(slash_command, guild_only, required_permissions = "ADMINISTRATOR")]
pub async fn delete(
    ctx: Context<'_>,
    #[description = "User whose avatar emoji to delete"] user: User,
) -> Result<()> {
    match avatarsync::delete(ctx.http(), &user, guild(ctx)).await? {
        Some(name) => {
            ctx.say(format!(
                "Deleted avatar emoji `:{name}:` for <@{}>.",
                user.id
            ))
            .await?;
        }
        None => {
            say_ephemeral(ctx, format!("No avatar emoji found for <@{}>.", user.id)).await?;
        }
    }
    Ok(())
}
