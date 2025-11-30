use crate::types::Context;
use color_eyre::eyre::Result;
use rand::Rng;

/// Rolls dice based on given # of sides
#[poise::command(slash_command)]
pub async fn roll(
    ctx: Context<'_>,
    #[description = "# of sides"] sides: Option<u32>,
) -> Result<()> {
    let sides = sides.unwrap_or(6);
    let roll = rand::rng().random_range(1..=sides);
    ctx.say(format!("You rolled a **{roll}**")).await?;
    Ok(())
}
