use crate::db;
use crate::starboard::{Config, store};
use crate::types::Context;
use color_eyre::eyre::Result;
use poise::CreateReply;
use poise::serenity_prelude::ChannelId;

/// Enable the starboard feature for this server
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn starboard_enable(
    ctx: Context<'_>,
    #[description = "Channel to post starred messages to"] channel: ChannelId,
    #[description = "Number of stars required to appear on starboard (default: 3)"]
    threshold: Option<i32>,
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

    let threshold = threshold.unwrap_or(3).clamp(1, 100);

    let config = Config {
        channel_id: channel.get(),
        threshold: threshold.cast_unsigned(),
    };
    db::with(|conn| store::set_config(conn, guild_id.get(), config))?;

    ctx.send(
        CreateReply::default()
            .content(format!(
                "✅ Starboard enabled for <#{channel}>! Messages with {threshold} or more ⭐ will be posted."
            ))
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// Disable the starboard feature for this server
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn starboard_disable(ctx: Context<'_>) -> Result<()> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.send(
            CreateReply::default()
                .content("This command can only be used in a server.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    db::with(|conn| store::delete_config(conn, guild_id.get()))?;

    ctx.send(
        CreateReply::default()
            .content("✅ Starboard disabled for this server.")
            .ephemeral(true),
    )
    .await?;

    Ok(())
}

/// Check the starboard configuration
#[poise::command(slash_command)]
pub async fn starboard_config(ctx: Context<'_>) -> Result<()> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.send(
            CreateReply::default()
                .content("This command can only be used in a server.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let config = db::with(|conn| store::config(conn, guild_id.get()))?;

    let response = if let Some(Config {
        channel_id,
        threshold,
    }) = config
    {
        format!(
            "⭐ **Starboard Configuration**\n- **Channel**: <#{channel_id}>\n- **Threshold**: {threshold} stars"
        )
    } else {
        "⭐ Starboard is not configured for this server. Use `/starboard_enable` to enable it."
            .to_string()
    };

    ctx.send(CreateReply::default().content(response).ephemeral(true))
        .await?;

    Ok(())
}
