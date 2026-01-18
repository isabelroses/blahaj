use color_eyre::eyre::Result;
use poise::serenity_prelude::{Context, FullEvent};

mod blahaj_is_this_true;
mod code_expantion;
mod replace_link;
mod starboard;

use crate::types::Data;

pub async fn event_handler(ctx: &Context, event: &FullEvent, data: &Data) -> Result<()> {
    if let FullEvent::Ready { data_about_bot } = event {
        println!("Logged in as {}", data_about_bot.user.name);
    }

    code_expantion::handle(ctx, event, data).await?;
    replace_link::handle(ctx, event, data).await?;
    blahaj_is_this_true::handle(ctx, event, data).await?;
    starboard::handle(ctx, event, data).await?;

    Ok(())
}
