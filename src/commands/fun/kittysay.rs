use color_eyre::eyre::Result;
use regex::Regex;
use std::process::Command;

use crate::Context;

/// Make the kitty say something :3
#[poise::command(slash_command, guild_only)]
pub async fn kittysay(ctx: Context<'_>, #[description = "speak"] input: String) -> Result<()> {
    let re = Regex::new(r"[^:a-zA-Z0-9\s]").unwrap();
    let sanitized_input = re.replace_all(&input, "").to_string();

    let output = Command::new("kittysay")
        .arg(&sanitized_input)
        .output()
        .expect("Failed to execute kittysay");

    ctx.say(format!("```{}```", String::from_utf8_lossy(&output.stdout)))
        .await?;
    Ok(())
}
