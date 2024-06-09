use crate::Context;
use color_eyre::eyre::Result;
use poise::{serenity_prelude as serenity, CreateReply};
use std::env;

/// Displays information about the bot
#[poise::command(slash_command)]
pub async fn botinfo(ctx: Context<'_>) -> Result<()> {
    let rev = env::var("BUILD_REV").unwrap_or("unknown".to_string());

    let embed = CreateReply::default().embed(
        serenity::CreateEmbed::default()
            .title("Bot Info")
            //.thumbnail(bot.avatar_url().expect("avatar failed"))
            .color(0xffffff)
            .field("Git rev", rev, false)
            .field("Bot ID", "1087418361283092510", false),
        //.field("Created at", bot.user.created_at().to_string(), false)
        //.field("Joined at", bot.joined_at.unwrap().to_string(), false),
    );

    ctx.send(embed).await?;
    Ok(())
}
