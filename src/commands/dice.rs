use crate::Context;
use color_eyre::eyre::Result;
use poise::{serenity_prelude as serenity, CreateReply};
use rand::Rng;

/// Rolls dice based on given # of sides
#[poise::command(slash_command)]
pub async fn roll(
    ctx: Context<'_>,
    #[description = "# of sides"] sides: Option<u32>,
) -> Result<()> {
    let sides = sides.unwrap_or(6);
    let roll = rand::thread_rng().gen_range(1..=sides);
    ctx.say(format!("You rolled a **{}**", roll)).await?;
    Ok(())
}
