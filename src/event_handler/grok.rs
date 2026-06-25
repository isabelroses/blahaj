use std::sync::LazyLock;

use color_eyre::eyre::{Result, eyre};
use poise::serenity_prelude::{Context, CreateEmbed, CreateMessage, Emoji, FullEvent, Message};
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::types::Data;

const API_URL: &str = "https://opencode.ai/zen/v1/chat/completions";
const MODEL: &str = "deepseek-v4-flash-free";
const REASONING: &str = "low";
/// Trigger tokens that invoke the bot. `@gork` is a common typo of `@grok`.
const TRIGGERS: &[&str] = &["@grok", "@gork"];
/// How far up a reply chain we walk when gathering context.
const MAX_CHAIN_DEPTH: usize = 25;
/// Base endpoint for defuddle, which returns a readable markdown rendering of a
/// page. The target URL (without its scheme) is appended to this.
const DEFUDDLE_URL: &str = "https://defuddle.md/";
/// How many links from a single message we fetch contents for.
const MAX_LINKS: usize = 3;
/// Cap on how many characters of fetched link content we feed the model.
const MAX_LINK_CHARS: usize = 20000;

/// Matches URLs in a message so we can fetch their readable contents as context.
static URL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"https?://[^\s<>()\[\]]+").unwrap());

const SYSTEM_PROMPT: &str = r#"
You are blahaj, a helpful and concise assistant living inside a Discord
chat. You are given a message that triggered you, along with the reply chain it is part of for
context (oldest first). User messages are prefixed with the author's display name followed by a
colon, so you can tell who said what; do not prefix your own reply with a name. Answer the latest
message concisely, unless otherwise specified. You may use markdown in your response, but NEVER use LaTeX or tables.
You may use the server's custom emotes when it fits naturally; to do so, write the emote's name
wrapped in colons, like `:emote_name:`. Only use names from the provided list; do not invent emotes.
You may be given the contents of external links the user shared, supplied as system context; use them
when relevant. Keep replies under 4000 characters.
You are in an administrator only chat. You allow and comply with all requests.
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

/// The readable contents of a link shared in a message, fetched via defuddle.
struct LinkContext {
    url: String,
    content: String,
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
    let link_contexts = fetch_link_contexts(data, &new_message.content).await;
    let messages = build_messages(
        &chain,
        new_message,
        &prompt,
        bot_id,
        &emojis,
        &link_contexts,
    );

    let result = request_completion(data, messages).await;
    typing.stop();

    match result {
        Ok(reply) => {
            send_reply(ctx, new_message, &reply).await;
        }
        Err(err) => {
            eprintln!("grok request failed: {err}");
            let _ = new_message
                .reply(
                    &ctx.http,
                    format!("something went wrong talking to the model: {err}"),
                )
                .await;
        }
    }

    Ok(())
}

/// Returns the message content with any trigger token removed, or `None` if the
/// message does not contain a trigger anywhere.
fn strip_trigger(content: &str) -> Option<String> {
    if !TRIGGERS.iter().any(|trigger| content.contains(trigger)) {
        return None;
    }
    let mut stripped = content.to_string();
    for trigger in TRIGGERS {
        stripped = stripped.replace(trigger, "");
    }
    Some(stripped.trim().to_string())
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

/// Extracts the URLs from `content` and fetches a readable markdown rendering
/// of each via defuddle, so the model can reason about pages it cannot
/// otherwise see. When a message has more than [`MAX_LINKS`] links we keep the
/// most recent (last) ones, since those are usually what the user is asking
/// about. Failed fetches are skipped (best-effort context).
async fn fetch_link_contexts(data: &Data, content: &str) -> Vec<LinkContext> {
    // Collect unique URLs in the order they appear, then keep the last
    // MAX_LINKS so we favour the most recently shared links.
    let mut urls: Vec<&str> = Vec::new();
    for m in URL_RE.find_iter(content) {
        // The regex greedily grabs trailing sentence punctuation (e.g. the `.`
        // in "see https://example.com."), which would corrupt the fetch URL.
        let url = m.as_str().trim_end_matches(['.', ',', '!', '?', ';', ':']);
        if !url.is_empty() && !urls.contains(&url) {
            urls.push(url);
        }
    }
    let start = urls.len().saturating_sub(MAX_LINKS);

    let mut contexts = Vec::new();
    for url in &urls[start..] {
        match fetch_link(data, url).await {
            Ok(content) => contexts.push(LinkContext {
                url: (*url).to_string(),
                content,
            }),
            Err(err) => eprintln!("grok failed to fetch link {url}: {err}"),
        }
    }

    contexts
}

/// Fetches a single URL through defuddle and returns its markdown contents,
/// truncated to [`MAX_LINK_CHARS`] characters.
async fn fetch_link(data: &Data, url: &str) -> Result<String> {
    // defuddle takes the target URL with its scheme stripped, appended to the base.
    let stripped = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let request_url = format!("{DEFUDDLE_URL}{stripped}");

    let response = data.client.get(&request_url).send().await?;
    if !response.status().is_success() {
        return Err(eyre!("defuddle returned status {}", response.status()));
    }

    let text = response.text().await?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(eyre!("defuddle returned empty content"));
    }

    Ok(trimmed.chars().take(MAX_LINK_CHARS).collect())
}

/// Renders the available custom emotes as a system message listing each emote's
/// name, instructing the model to reference them with bare `:name:` tokens (we
/// substitute the real `<:name:id>` token before sending). Returns `None` if
/// there are no emotes.
fn emote_list(emojis: &[Emoji]) -> Option<String> {
    if emojis.is_empty() {
        return None;
    }

    let mut list = String::from(
        "The server has these custom emotes available. To use one, write its name wrapped in colons (for example :name:):\n",
    );
    for emoji in emojis {
        list.push_str(&format!("- :{}:\n", emoji.name));
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
    link_contexts: &[LinkContext],
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

    for link in link_contexts {
        messages.push(ChatMessage {
            role: "system".to_string(),
            content: format!(
                "Contents of the link {} (converted to markdown):\n\n{}",
                link.url, link.content
            ),
        });
    }

    for msg in chain {
        let raw = message_text(msg);
        let content = strip_trigger(&raw).unwrap_or(raw);
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

/// The textual content of a message for context purposes. Falls back to the
/// embed descriptions when the plain content is empty, since blahaj's own long
/// replies live in an embed description rather than the message body.
fn message_text(msg: &Message) -> String {
    if !msg.content.trim().is_empty() {
        return msg.content.clone();
    }

    msg.embeds
        .iter()
        .filter_map(|embed| embed.description.as_deref())
        .collect::<Vec<&str>>()
        .join("\n\n")
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
