use color_eyre::eyre::Result;
use poise::serenity_prelude::{Context, FullEvent};

mod code_expantion;
mod kitten;

use crate::types::Data;

// TODO: add starboard
pub async fn event_handler(ctx: &Context, event: &FullEvent, data: &Data) -> Result<()> {
    if let FullEvent::Ready { data_about_bot } = event {
        println!("Logged in as {}", data_about_bot.user.name);
    }

    let client = &data.client;

    code_expantion::handle(ctx, event, client).await?;
    kitten::handle(ctx, event, client).await?;

    Ok(())
}
