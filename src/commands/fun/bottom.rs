use bottomify::bottom;
use color_eyre::eyre::Result;

use crate::Context;

/// Translate your words for the bottoms to understand
#[poise::command(slash_command, guild_only)]
pub async fn bottomify(ctx: Context<'_>, #[description = "text"] input: String) -> Result<()> {
    let out = bottom::encode_string(&input);

    ctx.say(format!("```{out}```")).await?;
    Ok(())
}

/// Translate your words for the tops and normies to understand
#[poise::command(slash_command, guild_only)]
pub async fn topify(ctx: Context<'_>, #[description = "text"] input: String) -> Result<()> {
    let out = bottom::decode_string(&input);

    if out.is_ok() {
        ctx.say(out?).await?;
    } else {
        ctx.say("I couldn't decode that message.").await?;
    }

    Ok(())
}
