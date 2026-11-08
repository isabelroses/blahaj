//! Messages the bot sends whose text might not fit in one Discord message.
//!
//! [`fit`] decides where long text goes (plain content, an embed, or an
//! attachment), and [`text_of`] reads it back from a message it produced.

use poise::serenity_prelude::{
    CreateAllowedMentions, CreateAttachment, CreateEmbed, CreateMessage, Message,
};

/// Discord's limit for plain message content, in characters.
const CONTENT_LIMIT: usize = 2000;
/// Discord's limit for an embed description, in characters.
const EMBED_LIMIT: usize = 4096;

/// What to do with text too long even for an embed.
#[derive(Clone, Copy)]
pub enum Overflow<'a> {
    /// Cut it to fit an embed, ending with `...`.
    Truncate,
    /// Show the start as content and attach the full text as a file.
    Attach { file_name: &'a str },
}

/// Fits `text` into one message: plain content when it's short enough,
/// otherwise an embed, otherwise whatever `overflow` says.
///
/// The message never pings anyone, since the text may come from users or a
/// model. Callers add a reply reference themselves if they want one.
pub fn fit(text: &str, overflow: Overflow<'_>) -> CreateMessage {
    let message = CreateMessage::new().allowed_mentions(CreateAllowedMentions::new());
    let length = text.chars().count();

    if length <= CONTENT_LIMIT {
        return message.content(text);
    }
    if length <= EMBED_LIMIT {
        return message.embed(CreateEmbed::new().description(text));
    }

    match overflow {
        Overflow::Truncate => {
            let truncated: String = text.chars().take(EMBED_LIMIT - 3).collect();
            message.embed(CreateEmbed::new().description(format!("{truncated}...")))
        }
        Overflow::Attach { file_name } => {
            let preview: String = text.chars().take(CONTENT_LIMIT).collect();
            message
                .content(preview)
                .add_file(CreateAttachment::bytes(text.as_bytes(), file_name))
        }
    }
}

/// The text of `msg`, including text [`fit`] moved into an embed.
pub fn text_of(msg: &Message) -> String {
    if !msg.content.trim().is_empty() {
        return msg.content.clone();
    }

    msg.embeds
        .iter()
        .filter_map(|embed| embed.description.as_deref())
        .collect::<Vec<&str>>()
        .join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn json(message: &CreateMessage) -> Value {
        serde_json::to_value(message).unwrap()
    }

    fn embed_description(message: &CreateMessage) -> String {
        json(message)["embeds"][0]["description"]
            .as_str()
            .unwrap()
            .to_string()
    }

    #[test]
    fn short_text_is_plain_content() {
        let message = fit("hi", Overflow::Truncate);
        assert_eq!(json(&message)["content"], "hi");
    }

    #[test]
    fn medium_text_goes_in_an_embed() {
        let text = "a".repeat(CONTENT_LIMIT + 1);
        let message = fit(&text, Overflow::Truncate);
        assert_eq!(embed_description(&message), text);
    }

    #[test]
    fn long_text_is_truncated_to_the_embed_limit() {
        let text = "é".repeat(EMBED_LIMIT + 10);
        let description = embed_description(&fit(&text, Overflow::Truncate));
        assert_eq!(description.chars().count(), EMBED_LIMIT);
        assert!(description.ends_with("..."));
    }

    #[test]
    fn long_text_can_be_attached_with_a_preview() {
        let text = "a".repeat(EMBED_LIMIT + 1);
        let message = fit(&text, Overflow::Attach { file_name: "x.md" });
        assert_eq!(
            json(&message)["content"].as_str().unwrap().len(),
            CONTENT_LIMIT
        );
    }

    #[test]
    fn never_pings() {
        let message = fit("@everyone <@1>", Overflow::Truncate);
        assert_eq!(
            json(&message)["allowed_mentions"]["parse"],
            Value::Array(vec![])
        );
    }
}
