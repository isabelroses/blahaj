use color_eyre::eyre::Result;
use poise::serenity_prelude::{Context, FullEvent};

use crate::Data;

// TODO: add more events
pub async fn event_handler(_ctx: &Context, event: &FullEvent, _data: &Data) -> Result<()> {
    if let FullEvent::Ready { data_about_bot } = event {
        println!("Logged in as {}", data_about_bot.user.name);
    }

    Ok(())
}
