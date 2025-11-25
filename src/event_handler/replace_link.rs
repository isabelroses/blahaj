use color_eyre::eyre::Result;
use poise::serenity_prelude::{Context, FullEvent};
use regex::Regex;
use serenity::all::EditMessage;

use crate::types::Data;

pub async fn handle(ctx: &Context, event: &FullEvent, _data: &Data) -> Result<()> {
    if let FullEvent::Message { new_message } = event {
        let regex = Regex::new(r"(https?:\/\/(?:(www|vm)\.)?(x\.com|twitter\.com|reddit\.com|instagram\.com|tiktok\.com)\/[^\s]+)").unwrap();
        let mut links: Vec<String> = Vec::new();
        let mut begging_no_twitter: bool = false;

        for capture in regex.find_iter(&new_message.content) {
            let url = capture.as_str();

            let modified_url = url
                .replace("https://x.com", "https://girlcockx.com")
                .replace("https://twitter.com", "https://fxtwitter.com")
                .replace("https://www.reddit.com", "https://rxddit.com")
                .replace("https://reddit.com", "https://rxddit.com")
                .replace("https://www.instagram.com", "https://kkinstagram.com")
                .replace("https://instagram.com", "https://kkinstagram.com")
                .replace("https://www.tiktok.com", "https://tnktok.com")
                .replace("https://vm.tiktok.com", "https://vm.tnktok.com")
                .replace("https://tiktok.com", "https://tnktok.com");

            if url.contains("x.com") || url.contains("twitter.com") {
                begging_no_twitter = true;
            }

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
            if begging_no_twitter {
                let _ = new_message
                    .reply(
                        ctx.http.clone(),
                        links.join("\n") + "\n-# Please stop using twitter!",
                    )
                    .await;
            } else {
                let _ = new_message.reply(ctx.http.clone(), links.join("\n")).await;
            }
        }
    }

    Ok(())
}
