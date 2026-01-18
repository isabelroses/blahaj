use crate::types::Context;
use color_eyre::eyre::Result;
use poise::CreateReply;
use poise::serenity_prelude::ChannelId;
use rusqlite::Connection;
use std::sync::{LazyLock, Mutex};

static STARBOARD_DB: LazyLock<Mutex<Connection>> = LazyLock::new(|| {
    let db_path = crate::utils::get_data_dir().join("starboard.db");
    Mutex::new(Connection::open(db_path).expect("Failed to open starboard database"))
});

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

    let threshold = threshold.unwrap_or(3).max(1).min(100);

    {
        let conn = STARBOARD_DB.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO starboard_config (guild_id, channel_id, threshold) VALUES (?, ?, ?)",
            [guild_id.get() as i64, channel.get() as i64, i64::from(threshold)],
        )?;
    }

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

    {
        let conn = STARBOARD_DB.lock().unwrap();
        conn.execute(
            "DELETE FROM starboard_config WHERE guild_id = ?",
            [guild_id.get() as i64],
        )?;
    }

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

    let config: Option<(u64, i32)> = {
        let conn = STARBOARD_DB.lock().unwrap();
        conn
            .query_row(
                "SELECT channel_id, threshold FROM starboard_config WHERE guild_id = ?",
                [guild_id.get() as i64],
                |row| {
                    let channel_id: i64 = row.get(0)?;
                    let threshold: i32 = row.get(1)?;
                    Ok((channel_id as u64, threshold))
                },
            )
            .ok()
    };

    let response = if let Some((channel_id, threshold)) = config {
        format!(
            "⭐ **Starboard Configuration**\n- **Channel**: <#{channel_id}>\n- **Threshold**: {threshold} stars"
        )
    } else {
        "⭐ Starboard is not configured for this server. Use `/starboard_enable` to enable it."
            .to_string()
    };

    ctx.send(
        CreateReply::default()
            .content(response)
            .ephemeral(true),
    )
    .await?;

    Ok(())
}
