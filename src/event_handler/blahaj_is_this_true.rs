use color_eyre::eyre::Result;
use poise::serenity_prelude::{Context, FullEvent};
use rand::Rng;
use regex::Regex;

use crate::types::Data;

const REPLIES: &[&str] = &[
    "Yes, absolutely!",
    "No, that's not true.",
    "Maybe, it depends on the context.",
    "Definitely not!",
    "Of course, it's true!",
    "I wouldn't bet on it.",
    "It's a possibility.",
    "Only if you believe it.",
];

pub async fn handle(ctx: &Context, event: &FullEvent, _data: &Data) -> Result<()> {
    if let FullEvent::Message { new_message } = event {
        if new_message.mentions_user(&ctx.cache.current_user()) {
            let regex = Regex::new(r"is this true(\?)?").unwrap();
            if regex.is_match(&new_message.content) {
                let select = rand::rng().random_range(0..=REPLIES.len());
                let response = REPLIES[select];

                // Reply with the response
                let _ = new_message.reply(&ctx.http, response).await;
            }
        }
    }

    Ok(())
}
