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
            let is_this = Regex::new(r"is this (true|real)(\?)?").unwrap();
            if is_this.is_match(&new_message.content) {
                let select = rand::rng().random_range(0..=REPLIES.len());
                let response = REPLIES[select];
                let _ = new_message.reply(&ctx.http, response).await;
            }

            let bomb = Regex::new(r"how.(to|do).*bomb").unwrap();
            if bomb.is_match(&new_message.content) {
                let response = "very carefully";
                let _ = new_message.reply(&ctx.http, response).await;
            }
        }
    }

    Ok(())
}
