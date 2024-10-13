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
    const MAX_LEN: usize = 1994;
    const WRAP: &str = "```";
    let out = bottom::decode_string(&input);

    if let Ok(out) = out {
        let mut out = out.as_str();
        let len = out.len();
        for _ in 0..(len / MAX_LEN) {
            let (x, xs) = out.split_at(MAX_LEN);
            ctx.say(format!("{WRAP}{x}{WRAP}")).await?;
            out = xs;
        }
        if len % MAX_LEN != 0 {
            ctx.say(format!("{WRAP}{out}{WRAP}")).await?;
        }
    } else {
        ctx.say("I couldn't decode that message.").await?;
    }

    Ok(())
}
