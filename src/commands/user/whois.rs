use crate::types::Context;
use color_eyre::eyre::Result;
use poise::{
    serenity_prelude::{CreateEmbed, User},
    CreateReply,
};

/// Displays your or another user's info
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn whois(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<User>,
) -> Result<()> {
    let user = user.as_ref().unwrap_or_else(|| ctx.author());
    let membership = ctx.guild_id().unwrap().member(ctx.http(), user.id).await?;

    let created_at = user.created_at().unix_timestamp();
    let joined_at = membership.joined_at.unwrap().unix_timestamp();

    let embed = CreateReply::default().embed(
        CreateEmbed::default()
            .title(&user.name)
            .thumbnail(user.avatar_url().expect("avatar failed"))
            .color(0x00ff_ffff)
            .field("ID", user.id.to_string(), false)
            .field("Username", &user.name, false)
            .field("Created at", format!("<t:{created_at}:R>"), false)
            .field("Joined at", format!("<t:{joined_at}:R>"), false)
            .field(
                "Roles",
                membership
                    .roles(ctx.cache())
                    .expect("No roles found")
                    .iter()
                    .map(|role| role.name.clone())
                    .collect::<Vec<String>>()
                    .join(", "),
                false,
            )
            .field("Bot", user.bot.to_string(), false),
    );

    ctx.send(embed).await?;
    Ok(())
}
