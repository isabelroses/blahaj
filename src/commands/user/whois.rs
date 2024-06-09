use crate::Context;
use color_eyre::eyre::Result;
use poise::{serenity_prelude as serenity, CreateReply};

/// Displays your or another user's info
#[poise::command(slash_command)]
pub async fn whois(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<()> {
    let user = user.as_ref().unwrap_or_else(|| ctx.author());
    let membership = ctx.guild_id().unwrap().member(ctx.http(), user.id).await?;

    let embed = CreateReply::default().embed(
        serenity::CreateEmbed::default()
            .title(&user.name)
            .thumbnail(user.avatar_url().expect("avatar failed"))
            .color(0x00ff_ffff)
            .field("ID", user.id.to_string(), false)
            .field("Username", &user.name, false)
            .field("Created at", user.created_at().to_string(), false)
            .field(
                "Joined at",
                membership.joined_at.expect("joined_at failed").to_string(),
                false,
            )
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
