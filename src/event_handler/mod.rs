use color_eyre::eyre::Result;
use poise::serenity_prelude::{Context, FullEvent};

mod code_expantion;

use crate::Data;

// TODO: add starboard
pub async fn event_handler(ctx: &Context, event: &FullEvent, _data: &Data) -> Result<()> {
    if let FullEvent::Ready { data_about_bot } = event {
        println!("Logged in as {}", data_about_bot.user.name);
    }

	let client = &_data.client;

    code_expantion::handle(ctx, event, client).await
}
