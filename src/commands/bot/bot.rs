use crate::types::Context;
use color_eyre::eyre::Result;
use poise::{
    serenity_prelude::{CreateEmbed, CreateEmbedAuthor},
    CreateReply,
};

/// Displays information about the bot
#[poise::command(slash_command)]
pub async fn botinfo(ctx: Context<'_>) -> Result<()> {
    let rev = option_env!("BUILD_REV").unwrap_or("unknown");

    let (bot_name, bot_face, bot_created_at) = {
        let bot = ctx.cache().current_user();
        (bot.name.clone(), bot.face().clone(), bot.created_at())
    };

    let embed = CreateReply::default().embed(
        CreateEmbed::default()
            .title("Bot Info")
            .author(CreateEmbedAuthor::new(bot_name).icon_url(&bot_face))
            .thumbnail(bot_face)
            .color(0x00ff_ffff)
            .field("Git rev", rev, false)
            .field("Bot ID", "1087418361283092510", false)
            .field("Created at", bot_created_at.to_string(), false),
    );

    ctx.send(embed).await?;
    Ok(())
}
