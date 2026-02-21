use color_eyre::eyre::{Result, eyre};
use once_cell::sync::Lazy;
use poise::serenity_prelude::{ChannelId, GuildId, MessageId};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::types::Context;

static DISCORD_MESSAGE_LINK_REGEX: std::sync::LazyLock<Regex> = std::sync::LazyLock::new(|| {
    Regex::new(r"^https?://(?:canary\.|ptb\.)?discord\.com/channels/(\d+)/(\d+)/(\d+)$")
        .expect("valid discord message link regex")
});

#[derive(Debug)]
struct ParsedMessageLink {
    guild_id: GuildId,
    channel_id: ChannelId,
    message_id: MessageId,
}

#[derive(Serialize)]
struct KagiTranslateRequest<'a> {
    from: &'a str,
    to: &'a str,
    text: &'a str,
    stream: bool,
    prediction: &'a str,
    formality: &'a str,
    speaker_gender: &'a str,
    addressee_gender: &'a str,
    translation_style: &'a str,
    context: &'a str,
    model: &'a str,
    dictionary_language: &'a str,
}

#[derive(Deserialize)]
struct KagiTranslateResponse {
    translation: String,
}

fn parse_discord_message_link(link: &str) -> Option<ParsedMessageLink> {
    let captures = DISCORD_MESSAGE_LINK_REGEX.captures(link)?;
    let guild_id = captures.get(1)?.as_str().parse::<u64>().ok()?;
    let channel_id = captures.get(2)?.as_str().parse::<u64>().ok()?;
    let message_id = captures.get(3)?.as_str().parse::<u64>().ok()?;

    Some(ParsedMessageLink {
        guild_id: GuildId::new(guild_id),
        channel_id: ChannelId::new(channel_id),
        message_id: MessageId::new(message_id),
    })
}

/// Translate Gen Z text into normal US English
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn genz(
    ctx: Context<'_>,
    #[description = "gen z text to translate"] text: Option<String>,
    #[description = "discord message link to translate"] message_link: Option<String>,
) -> Result<()> {
    let Some(cookie) = ctx.data().kagi_cookie.as_deref() else {
        ctx.say("This command is not configured: missing `KAGI_COOKIE`.")
            .await?;
        return Ok(());
    };

    if text.is_some() && message_link.is_some() {
        ctx.say("Use either `text` or `message_link`, not both.")
            .await?;
        return Ok(());
    }

    let input = if let Some(input_text) = text {
        input_text
    } else if let Some(link) = message_link {
        let parsed = parse_discord_message_link(&link)
            .ok_or_else(|| eyre!("Invalid Discord message link format"))?;

        let Some(current_guild_id) = ctx.guild_id() else {
            ctx.say("Message links can only be used from inside a server command context.")
                .await?;
            return Ok(());
        };

        if parsed.guild_id != current_guild_id {
            ctx.say("That message link is outside this server; refusing to fetch it.")
                .await?;
            return Ok(());
        }

        let message = parsed
            .channel_id
            .message(ctx.serenity_context(), parsed.message_id)
            .await
            .map_err(|e| eyre!("Failed to fetch message from link: {e}"))?;

        if message.content.trim().is_empty() {
            ctx.say("The linked message has no text content to translate.")
                .await?;
            return Ok(());
        }

        message.content
    } else {
        ctx.say("Provide either `text` or `message_link`.").await?;
        return Ok(());
    };

    const MAX_INPUT_CHARS: usize = 2_000;
    let input_char_count = input.chars().count();
    if input_char_count > MAX_INPUT_CHARS {
        ctx.say(format!(
            "Input is too long ({input_char_count} chars). Max is {MAX_INPUT_CHARS} chars."
        ))
        .await?;
        return Ok(());
    }

    ctx.defer().await?;

    let payload = KagiTranslateRequest {
        from: "gen_z",
        to: "en_us",
        text: input.as_str(),
        stream: false,
        prediction: "",
        formality: "default",
        speaker_gender: "unknown",
        addressee_gender: "unknown",
        translation_style: "natural",
        context: "",
        model: "standard",
        dictionary_language: "en",
    };

    let response = ctx
        .data()
        .client
        .post("https://translate.kagi.com/api/translate")
        .header("accept", "*/*")
        .header("content-type", "application/json")
        .header("origin", "https://translate.kagi.com")
        .header("referer", "https://translate.kagi.com/")
        .header("cookie", cookie)
        .json(&payload)
        .send()
        .await
        .map_err(|e| eyre!("Failed to call Kagi Translate: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_else(|_| String::new());
        return Err(eyre!("Kagi Translate error ({status}): {body}"));
    }

    let body: KagiTranslateResponse = response
        .json()
        .await
        .map_err(|e| eyre!("Failed to parse Kagi Translate response: {e}"))?;

    ctx.say(format!("> {}\n\"{}\"", input, body.translation))
        .await?;
    Ok(())
}
