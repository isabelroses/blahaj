use crate::Context;
use color_eyre::eyre::Result;
use poise::{serenity_prelude::CreateEmbed, CreateReply};

/// Displays information about the bot
#[poise::command(slash_command)]
pub async fn botinfo(ctx: Context<'_>) -> Result<()> {
    let rev = option_env!("BUILD_REV").unwrap_or("unknown");

    let embed = CreateReply::default().embed(
        CreateEmbed::default()
            .title("Bot Info")
            //.thumbnail(bot.avatar_url().expect("avatar failed"))
            .color(0x00ff_ffff)
            .field("Git rev", rev, false)
            .field("Bot ID", "1087418361283092510", false),
        //.field("Created at", bot.user.created_at().to_string(), false)
        //.field("Joined at", bot.joined_at.unwrap().to_string(), false),
    );

    ctx.send(embed).await?;
    Ok(())
}
