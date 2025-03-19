use color_eyre::eyre::Result;
use poise::serenity_prelude::{Context, FullEvent};
use regex::Regex;
use serenity::all::EditMessage;

use crate::types::Data;

pub async fn handle(ctx: &Context, event: &FullEvent, _data: &Data) -> Result<()> {
    if let FullEvent::Message { new_message } = event {
        let regex = Regex::new(r"(https?:\/\/(?:www\.)?(x\.com|twitter\.com|reddit\.com|instagram\.com|tiktok\.com)\/[^\s]+)").unwrap();
        let mut links: Vec<String> = Vec::new();

        for capture in regex.find_iter(&new_message.content) {
            let url = capture.as_str();

            let modified_url = url
                .replace("https://x.com", "https://fxtwitter.com")
                .replace("https://twitter.com", "https://fxtwitter.com")
                .replace("https://www.reddit.com", "https://rxddit.com")
                .replace("https://reddit.com", "https://rxddit.com")
                .replace("https://www.instagram.com", "https://ddinstagram.com")
                .replace("https://instagram.com", "https://ddinstagram.com")
                .replace("https://www.tiktok.com", "https://tfxktok.com")
                .replace("https://tiktok.com", "https://tfxktok.com");

            links.push(modified_url);
        }

        let message_id = new_message.id;
        let channel_id = new_message.channel_id;

        if !links.is_empty() {
            let _ = channel_id
                .edit_message(
                    ctx.http.clone(),
                    message_id,
                    EditMessage::new().suppress_embeds(true),
                )
                .await;
            let _ = new_message.reply(ctx.http.clone(), links.join("\n")).await;
        }
    }

    Ok(())
}
