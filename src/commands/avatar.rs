use crate::Context;
use color_eyre::eyre::Result;
use poise::serenity_prelude as serenity;

/// Displays your or another user's avatar
#[poise::command(slash_command)]
pub async fn avatar(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<serenity::User>,
) -> Result<()> {
    let u = user.as_ref().unwrap_or_else(|| ctx.author());
    let av = u.avatar_url().expect("Could not get avatar URL");
    ctx.say(av).await?;
    Ok(())
}
