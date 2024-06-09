use color_eyre::eyre::Result;

use crate::Context;

/// Replies with "Pong!" or does it?
#[poise::command(slash_command, guild_only)]
pub async fn ping(ctx: Context<'_>) -> Result<()> {
    let ping = humantime::format_duration(ctx.ping().await);
    ctx.say(format!("Pong! `{ping}`")).await?;
    Ok(())
}
