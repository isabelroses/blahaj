use crate::Context;
use color_eyre::eyre::Result;
use poise::serenity_prelude::User;

/// Displays your or another user's avatar
#[poise::command(slash_command)]
pub async fn avatar(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<User>,
) -> Result<()> {
    let user = user.as_ref().unwrap_or_else(|| ctx.author());
    let avatar = user.avatar_url().expect("Could not get avatar URL");
    ctx.say(avatar).await?;
    Ok(())
}
