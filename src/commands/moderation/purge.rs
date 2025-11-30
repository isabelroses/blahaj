use crate::Result;
use crate::types::Context;
use poise::serenity_prelude::GetMessages;

#[poise::command(slash_command, guild_only, required_permissions = "BAN_MEMBERS")]
pub async fn purge(ctx: Context<'_>, messages_count: Option<u8>) -> Result<()> {
    let amount = messages_count.unwrap_or(10);
    let current_channel = ctx.channel_id();
    for message in current_channel
        .messages(&ctx, GetMessages::new().before(ctx.id()).limit(amount))
        .await?
    {
        message.delete(&ctx).await?;
    }

    ctx.say(format!(
        "Successfully purged `{amount}` messages from this channel"
    ))
    .await?;
    Ok(())
}
