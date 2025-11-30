use crate::types::Context;
use color_eyre::eyre::Result;
use poise::serenity_prelude::User;
use rand::Rng;

#[poise::command(slash_command)]
pub async fn height(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<User>,
) -> Result<()> {
    let user = user.as_ref().unwrap_or_else(|| ctx.author());

    let (feet, inches, cm) = match user.id.get() {
        463566237918691338 => (8, 5, 256),
        474274492810788864 => (4, 1, 124),
        _ => {
            let total_inches = rand::rng().random_range(49..=101);
            let feet = total_inches / 12;
            let inches = total_inches % 12;
            let cm = (total_inches as f32 * 2.54) as u32;
            (feet, inches, cm)
        }
    };

    ctx.say(format!(
        "🔮 **{}** is **{}'{}\"** (**{} cm**) tall!",
        user.display_name(),
        feet,
        inches,
        cm
    ))
    .await?;

    Ok(())
}
