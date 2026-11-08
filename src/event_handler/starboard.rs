use color_eyre::eyre::Result;
use poise::serenity_prelude::{
    ChannelId, Colour, Context, CreateMessage, EditMessage, FullEvent, Message, MessageId,
    Reaction, ReactionType,
};

use crate::db;
use crate::starboard::{self, Action};
use crate::types::Data;

pub async fn handle(ctx: &Context, event: &FullEvent, _data: &Data) -> Result<()> {
    match event {
        FullEvent::ReactionAdd { add_reaction } => on_reaction(ctx, add_reaction, true).await,
        FullEvent::ReactionRemove { removed_reaction } => {
            on_reaction(ctx, removed_reaction, false).await
        }
        _ => Ok(()),
    }
}

const STAR: &str = "⭐";

/// Compares against the inner string rather than building a `ReactionType` to
/// compare with, which would allocate on every reaction event.
fn is_star(reaction_type: &ReactionType) -> bool {
    matches!(reaction_type, ReactionType::Unicode(emoji) if emoji == STAR)
}

/// Carries out a star change on Discord: reads the current star count, asks
/// [`starboard`] what to do, and reports back how it went.
async fn on_reaction(ctx: &Context, reaction: &Reaction, added: bool) -> Result<()> {
    if !is_star(&reaction.emoji) {
        return Ok(());
    }
    let Some(guild_id) = reaction.guild_id else {
        return Ok(());
    };
    let Some(config) = db::with(|conn| starboard::store::config(conn, guild_id.get()))? else {
        return Ok(());
    };

    let message = reaction
        .channel_id
        .message(ctx, reaction.message_id)
        .await?;

    // Don't let the starboard's own posts be starred onto the starboard.
    if added && message.author.bot && reaction.channel_id.get() == config.channel_id {
        return Ok(());
    }

    let star_count = message
        .reactions
        .iter()
        .find(|r| is_star(&r.reaction_type))
        .map_or(0, |r| r.count);
    let message_id = reaction.message_id.get();

    let action = db::with(|conn| {
        if added {
            starboard::on_star_added(
                conn,
                config,
                message_id,
                guild_id.get(),
                reaction.channel_id.get(),
                star_count,
            )
        } else {
            starboard::on_star_removed(conn, config, message_id, star_count)
        }
    })?;

    let starboard_channel = ChannelId::new(config.channel_id);
    match action {
        Action::Nothing => {}
        Action::Edit(id) => {
            if let Ok(mut post) = starboard_channel.message(ctx, MessageId::new(id)).await {
                let embed = create_star_embed(ctx, &message, star_count).await;
                post.edit(ctx, EditMessage::new().embed(embed)).await.ok();
            }
        }
        Action::Post => {
            let embed = create_star_embed(ctx, &message, star_count).await;
            let link = format!(
                "https://discord.com/channels/{guild_id}/{}/{}",
                reaction.channel_id, reaction.message_id
            );
            let posted = starboard_channel
                .send_message(ctx, CreateMessage::new().embed(embed).content(link))
                .await
                .ok()
                .map(|post| post.id.get());
            db::with(|conn| starboard::finish_post(conn, message_id, posted, star_count))?;
        }
        Action::Delete(id) => {
            let deleted = starboard_channel
                .delete_message(ctx, MessageId::new(id))
                .await
                .is_ok();
            db::with(|conn| starboard::finish_delete(conn, message_id, deleted, star_count))?;
        }
    }

    Ok(())
}

async fn create_star_embed(
    ctx: &Context,
    message: &Message,
    star_count: u64,
) -> poise::serenity_prelude::CreateEmbed {
    let mut embed = poise::serenity_prelude::CreateEmbed::default()
        .author(
            poise::serenity_prelude::CreateEmbedAuthor::new(&message.author.name)
                .icon_url(message.author.face()),
        )
        .description(&message.content)
        .footer(poise::serenity_prelude::CreateEmbedFooter::new(format!(
            "⭐ {star_count}"
        )))
        .colour(Colour::GOLD)
        .timestamp(message.timestamp);

    if let Some(message_reference) = &message.message_reference
        && let Some(reference_message_id) = message_reference.message_id
    {
        let reference_details =
            if let Some(reference_message) = message.referenced_message.as_deref() {
                Some((
                    reference_message.author.name.clone(),
                    summarized_message_content(
                        &reference_message.content,
                        !reference_message.attachments.is_empty(),
                    ),
                ))
            } else {
                let reference_channel_id = message_reference.channel_id;
                reference_channel_id
                    .message(ctx, reference_message_id)
                    .await
                    .ok()
                    .map(|reference_message| {
                        (
                            reference_message.author.name,
                            summarized_message_content(
                                &reference_message.content,
                                !reference_message.attachments.is_empty(),
                            ),
                        )
                    })
            };

        if let Some((reference_author_name, reference_preview)) = reference_details {
            let reference_channel_id = message_reference.channel_id;
            let reference_url = format!(
                "https://discord.com/channels/{}/{}/{}",
                message
                    .guild_id
                    .map_or_else(|| "@me".to_string(), |guild_id| guild_id.get().to_string()),
                reference_channel_id,
                reference_message_id
            );

            embed = embed.field(
                "Replying to",
                format!(
                    "**{reference_author_name}**\n{reference_preview}\n[Jump to referenced message]({reference_url})"
                ),
                false,
            );
        }
    }

    if let Some(first_attachment) = message.attachments.first() {
        embed = embed.image(&first_attachment.url);
    }

    embed
}

fn summarized_message_content(content: &str, has_attachments: bool) -> String {
    let collapsed = content.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.is_empty() {
        return if has_attachments {
            "[attachment-only message]".to_string()
        } else {
            "[no text content]".to_string()
        };
    }

    let preview_limit = 300;
    let char_count = collapsed.chars().count();
    if char_count > preview_limit {
        let mut truncated = collapsed.chars().take(preview_limit).collect::<String>();
        truncated.push('…');
        truncated
    } else {
        collapsed
    }
}
