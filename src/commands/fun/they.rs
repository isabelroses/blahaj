use color_eyre::eyre::Result;

use crate::types::Context;

/// they
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn they(ctx: Context<'_>) -> Result<()> {
    ctx.say("https://media.discordapp.net/attachments/1250587586217639946/1434205305029394552/20251101_113918.jpg?ex=69140186&is=6912b006&hm=366bebabba1c5dbc5cefd96ea9ddf73b3a2e3a594ba95f61b4023cc0d065cfab&=&format=webp".to_string()).await?;
    Ok(())
}
