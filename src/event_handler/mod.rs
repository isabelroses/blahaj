use color_eyre::eyre::Result;
use poise::serenity_prelude::{Context, FullEvent};

mod avatarsync;
mod blahaj_is_this_true;
mod code_expantion;
// Disabled in #32; kept so it can be switched back on.
#[allow(dead_code)]
mod grok;
mod replace_link;
mod starboard;

use crate::types::Data;

pub async fn event_handler(ctx: &Context, event: &FullEvent, data: &Data) -> Result<()> {
    if let FullEvent::Ready { data_about_bot } = event {
        println!("Logged in as {}", data_about_bot.user.name);
    }

    // Run the handlers concurrently so a slow one doesn't delay the rest, and
    // an error in one doesn't stop the others.
    tokio::join!(
        logged("code_expantion", code_expantion::handle(ctx, event, data)),
        logged("replace_link", replace_link::handle(ctx, event, data)),
        logged(
            "blahaj_is_this_true",
            blahaj_is_this_true::handle(ctx, event, data)
        ),
        logged("avatarsync", avatarsync::handle(ctx, event, data)),
        // logged("grok", grok::handle(ctx, event, data)),
        logged("starboard", starboard::handle(ctx, event, data)),
    );

    Ok(())
}

async fn logged(name: &str, handler: impl Future<Output = Result<()>>) {
    if let Err(err) = handler.await {
        eprintln!("event handler {name} failed: {err:?}");
    }
}
