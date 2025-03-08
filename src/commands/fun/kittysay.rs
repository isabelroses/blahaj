use color_eyre::eyre::Result;
use kittysay::{print, FormatOptions};

use crate::types::Context;

/// Make the kitty say something :3
#[poise::command(slash_command, guild_only)]
pub async fn kittysay(
    ctx: Context<'_>,
    #[description = "say"] input: String,
    #[description = "think"] think: Option<bool>,
) -> Result<()> {
    let opts = FormatOptions {
        think: think.unwrap_or(false),
        width: 45,
    };

    let output = print(&input, &opts);

    ctx.say(format!("```{output}```")).await?;
    Ok(())
}
