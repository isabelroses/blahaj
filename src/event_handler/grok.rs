use color_eyre::eyre::{Result, eyre};
use poise::serenity_prelude::{Context, CreateEmbed, CreateMessage, Emoji, FullEvent, Message};
use serde::{Deserialize, Serialize};

use crate::types::Data;

const API_URL: &str = "https://opencode.ai/zen/v1/chat/completions";
const MODEL: &str = "deepseek-v4-flash-free";
const REASONING: &str = "xhigh";
const TRIGGER: &str = "@grok";
/// How far up a reply chain we walk when gathering context.
const MAX_CHAIN_DEPTH: usize = 25;

const SYSTEM_PROMPT: &str = r#"
You are blahaj, a helpful and concise assistant living inside a Discord
chat. You are given a message that triggered you, along with the reply chain it is part of for
context (oldest first). User messages are prefixed with the author's display name followed by a
colon, so you can tell who said what; do not prefix your own reply with a name. Answer the latest
message concisely, unless otherwise specified. You may use markdown in your response, but NEVER use LaTeX or tables.
You may use the server's custom emotes when it fits naturally; to do so, copy one of the provided
emote tokens verbatim (including the angle brackets). Do not invent emote tokens or guess their ids.
Keep replies under 2000 characters.
"#;

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    reasoning: &'a str,
    messages: Vec<ChatMessage>,
}

#[derive(Serialize, Deserialize, Clone)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ChatMessage,
}

pub async fn handle(ctx: &Context, event: &FullEvent, data: &Data) -> Result<()> {
    let FullEvent::Message { new_message } = event else {
        return Ok(());
    };

    // Ignore messages from bots (including ourselves) to avoid loops.
    if new_message.author.bot {
        return Ok(());
    }

    let bot_id = ctx.cache.current_user().id;

    // Respond when the message mentions the trigger anywhere, or when it is a
    // reply to one of our own messages.
    let (prompt, explicit) = match strip_trigger(&new_message.content) {
        Some(prompt) => (prompt, true),
        None if is_reply_to_bot(new_message, bot_id) => {
            (new_message.content.trim().to_string(), false)
        }
        None => return Ok(()),
    };

    if prompt.is_empty() && explicit {
        let _ = new_message
            .reply(&ctx.http, "ask me something after `@grok`")
            .await;
        return Ok(());
    }

    // Keeps the typing indicator alive (re-broadcast every few seconds) until
    // dropped, so it persists across slow model responses.
    let typing = new_message.channel_id.start_typing(&ctx.http);

    let emojis = fetch_emojis(ctx, new_message).await;
    let chain = collect_chain(ctx, new_message).await;
    let messages = build_messages(&chain, new_message, &prompt, bot_id, &emojis);

    let result = request_completion(data, messages).await;
    typing.stop();

    match result {
        Ok(reply) => {
            send_reply(ctx, new_message, &reply).await;
        }
        Err(err) => {
            eprintln!("grok request failed: {err}");
            let _ = new_message
                .reply(&ctx.http, "something went wrong talking to the model")
                .await;
        }
    }

    Ok(())
}

/// Returns the message content with the `@grok` trigger removed, or `None` if
/// the message does not contain the trigger anywhere.
fn strip_trigger(content: &str) -> Option<String> {
    if !content.contains(TRIGGER) {
        return None;
    }
    Some(content.replace(TRIGGER, "").trim().to_string())
}

/// Whether `msg` is a reply to a message authored by the bot.
fn is_reply_to_bot(msg: &Message, bot_id: poise::serenity_prelude::UserId) -> bool {
    msg.referenced_message
        .as_ref()
        .is_some_and(|parent| parent.author.id == bot_id)
}

/// Fetches the custom emotes for the guild the message was sent in. Returns an
/// empty list in DMs or if the request fails (emotes are best-effort context).
async fn fetch_emojis(ctx: &Context, msg: &Message) -> Vec<Emoji> {
    let Some(guild_id) = msg.guild_id else {
        return Vec::new();
    };

    match guild_id.emojis(&ctx.http).await {
        Ok(emojis) => emojis,
        Err(err) => {
            eprintln!("grok failed to fetch emojis: {err}");
            Vec::new()
        }
    }
}

/// Renders the available custom emotes as a system message listing each emote's
/// name alongside the exact token the model must copy to use it, or `None` if
/// there are no emotes.
fn emote_list(emojis: &[Emoji]) -> Option<String> {
    if emojis.is_empty() {
        return None;
    }

    let mut list = String::from(
        "The server has these custom emotes available. To use one, copy its token verbatim:\n",
    );
    for emoji in emojis {
        // `Emoji`'s Display renders the `<:name:id>` token Discord expects.
        list.push_str(&format!("- {}: {}\n", emoji.name, emoji));
    }
    Some(list)
}

/// Walks up the reply chain starting from (but not including) `start`,
/// returning the referenced messages ordered oldest first.
async fn collect_chain(ctx: &Context, start: &Message) -> Vec<Message> {
    let mut chain = Vec::new();
    let mut current = start.clone();

    while chain.len() < MAX_CHAIN_DEPTH {
        let Some(reference) = &current.message_reference else {
            break;
        };
        let Some(message_id) = reference.message_id else {
            break;
        };

        match current.channel_id.message(&ctx.http, message_id).await {
            Ok(parent) => {
                current = parent.clone();
                chain.push(parent);
            }
            Err(_) => break,
        }
    }

    chain.reverse();
    chain
}

/// Builds the message list sent to the model: a system prompt, the reply chain
/// as context, and the triggering message last. User messages are prefixed with
/// the author's username so the model knows who said what.
fn build_messages(
    chain: &[Message],
    trigger: &Message,
    prompt: &str,
    bot_id: poise::serenity_prelude::UserId,
    emojis: &[Emoji],
) -> Vec<ChatMessage> {
    let mut messages = Vec::with_capacity(chain.len() + 3);

    messages.push(ChatMessage {
        role: "system".to_string(),
        content: SYSTEM_PROMPT.to_string(),
    });

    if let Some(list) = emote_list(emojis) {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: list,
        });
    }

    for msg in chain {
        let content = strip_trigger(&msg.content).unwrap_or_else(|| msg.content.clone());
        if content.trim().is_empty() {
            continue;
        }

        if msg.author.id == bot_id {
            messages.push(ChatMessage {
                role: "assistant".to_string(),
                content: content.clone(),
            });
        } else {
            messages.push(ChatMessage {
                role: "user".to_string(),
                content: format!("{}: {}", display_name(msg), content),
            });
        }
    }

    messages.push(ChatMessage {
        role: "user".to_string(),
        content: format!("{}: {}", display_name(trigger), prompt),
    });

    messages
}

/// The name to attribute a message to: the per-guild nickname if present,
/// otherwise the global display name, otherwise the username.
fn display_name(msg: &Message) -> &str {
    msg.member
        .as_ref()
        .and_then(|member| member.nick.as_deref())
        .or(msg.author.global_name.as_deref())
        .unwrap_or(&msg.author.name)
}

async fn request_completion(data: &Data, messages: Vec<ChatMessage>) -> Result<String> {
    let body = ChatRequest {
        model: MODEL,
        reasoning: REASONING,
        messages,
    };

    let response = data.client.post(API_URL).json(&body).send().await?;

    if !response.status().is_success() {
        return Err(eyre!("model returned status {}", response.status()));
    }

    let parsed: ChatResponse = response.json().await?;
    let content = parsed
        .choices
        .into_iter()
        .next()
        .map(|choice| choice.message.content)
        .ok_or_else(|| eyre!("model returned no choices"))?;

    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Err(eyre!("model returned empty content"));
    }

    Ok(trimmed.to_string())
}

/// Replies to `message`, using an embed when the response is too long for a
/// regular message and truncating if it exceeds the embed limit too.
async fn send_reply(ctx: &Context, message: &Message, reply: &str) {
    let length = reply.chars().count();

    if length <= 2000 {
        let _ = message.reply(&ctx.http, reply).await;
    } else if length <= 4096 {
        let builder = CreateMessage::new()
            .embed(CreateEmbed::new().description(reply))
            .reference_message(message);
        let _ = message.channel_id.send_message(&ctx.http, builder).await;
    } else {
        let truncated: String = reply.chars().take(4093).collect();
        let builder = CreateMessage::new()
            .embed(CreateEmbed::new().description(format!("{truncated}...")))
            .reference_message(message);
        let _ = message.channel_id.send_message(&ctx.http, builder).await;
    }
}
