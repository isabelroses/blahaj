use color_eyre::eyre::Result;
use poise::serenity_prelude::{Colour, Context, EditMessage, FullEvent, ReactionType};

use crate::types::Data;
use crate::utils::STARBOARD_DB;

pub async fn handle(ctx: &Context, event: &FullEvent, _data: &Data) -> Result<()> {
    match event {
        FullEvent::ReactionAdd { add_reaction } => {
            handle_reaction_add(ctx, add_reaction).await?;
        }
        FullEvent::ReactionRemove { removed_reaction } => {
            handle_reaction_remove(ctx, removed_reaction).await?;
        }
        _ => {}
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn handle_reaction_add(
    ctx: &Context,
    reaction: &poise::serenity_prelude::Reaction,
) -> Result<()> {
    // Only handle star reactions
    if reaction.emoji != ReactionType::Unicode("⭐".to_string()) {
        return Ok(());
    }

    let Some(guild_id) = reaction.guild_id else {
        return Ok(());
    };

    // Get starboard config
    let config: Option<(u64, u32)> = {
        let conn = STARBOARD_DB.lock().unwrap();
        conn.query_row(
            "SELECT channel_id, threshold FROM starboard_config WHERE guild_id = ?",
            [guild_id.get().cast_signed()],
            |row| {
                let channel_id: i64 = row.get(0)?;
                let threshold: i32 = row.get(1)?;
                Ok((channel_id.cast_unsigned(), threshold.cast_unsigned()))
            },
        )
        .ok()
    };

    let Some((starboard_channel_id, threshold)) = config else {
        return Ok(());
    };

    // Get the message
    let message = reaction
        .channel_id
        .message(ctx, reaction.message_id)
        .await?;

    // Don't allow starring bot's starboard messages
    if message.author.bot && reaction.channel_id.get() == starboard_channel_id {
        return Ok(());
    }

    // Count star reactions
    let star_count = message
        .reactions
        .iter()
        .find(|r| r.reaction_type == ReactionType::Unicode("⭐".to_string()))
        .map_or(0, |r| r.count);

    if star_count < u64::from(threshold) {
        return Ok(());
    }

    let mut should_send = false;
    let mut edit_starboard_msg_id: Option<i64> = None;

    {
        let conn = STARBOARD_DB.lock().unwrap();
        let existing: Option<(Option<i64>, i64)> = conn
            .query_row(
                "SELECT starboard_message_id, posting FROM starred_messages WHERE message_id = ?",
                [reaction.message_id.get().cast_signed()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();

        if let Some((starboard_msg_id, posting)) = existing {
            conn.execute(
                "UPDATE starred_messages SET star_count = ? WHERE message_id = ?",
                [
                    star_count.cast_signed(),
                    reaction.message_id.get().cast_signed(),
                ],
            )
            .ok();

            if let Some(starboard_msg_id) = starboard_msg_id {
                edit_starboard_msg_id = Some(starboard_msg_id);
            } else if posting == 0 {
                let updated = conn
                    .execute(
                        "UPDATE starred_messages SET posting = 1 WHERE message_id = ?",
                        [reaction.message_id.get().cast_signed()],
                    )
                    .ok();
                should_send = matches!(updated, Some(rows) if rows > 0);
            }
        } else {
            let inserted = conn
                .execute(
                "INSERT INTO starred_messages (message_id, guild_id, channel_id, starboard_message_id, star_count, posting) VALUES (?, ?, ?, NULL, ?, 1)",
                [
                    reaction.message_id.get().cast_signed(),
                    guild_id.get().cast_signed(),
                    reaction.channel_id.get().cast_signed(),
                    star_count.cast_signed(),
                ],
                )
                .ok();
            should_send = matches!(inserted, Some(rows) if rows > 0);
        }
    }

    if let Some(starboard_msg_id) = edit_starboard_msg_id
        && let Ok(mut starboard_msg) = poise::serenity_prelude::ChannelId::new(starboard_channel_id)
            .message(
                ctx,
                poise::serenity_prelude::MessageId::new(starboard_msg_id.cast_unsigned()),
            )
            .await
    {
        let embed = create_star_embed(ctx, &message, star_count).await;
        starboard_msg
            .edit(ctx, EditMessage::new().embed(embed))
            .await
            .ok();
    }

    if should_send {
        // Create starboard message
        let starboard_channel = poise::serenity_prelude::ChannelId::new(starboard_channel_id);
        let embed = create_star_embed(ctx, &message, star_count).await;

        if let Ok(starboard_msg) = starboard_channel
            .send_message(
                ctx,
                poise::serenity_prelude::CreateMessage::new()
                    .embed(embed)
                    .content(format!(
                        "https://discord.com/channels/{}/{}/{}",
                        guild_id, reaction.channel_id, reaction.message_id
                    )),
            )
            .await
        {
            let conn = STARBOARD_DB.lock().unwrap();
            conn.execute(
                "UPDATE starred_messages SET starboard_message_id = ?, posting = 0, star_count = ? WHERE message_id = ?",
                [
                    starboard_msg.id.get().cast_signed(),
                    star_count.cast_signed(),
                    reaction.message_id.get().cast_signed(),
                ],
            )
            .ok();
        } else {
            let conn = STARBOARD_DB.lock().unwrap();
            conn.execute(
                "UPDATE starred_messages SET posting = 0 WHERE message_id = ?",
                [reaction.message_id.get().cast_signed()],
            )
            .ok();
        }
    }

    Ok(())
}

async fn handle_reaction_remove(
    ctx: &Context,
    reaction: &poise::serenity_prelude::Reaction,
) -> Result<()> {
    // Only handle star reactions
    if reaction.emoji != ReactionType::Unicode("⭐".to_string()) {
        return Ok(());
    }

    let Some(guild_id) = reaction.guild_id else {
        return Ok(());
    };

    // Get starboard config
    let config: Option<(u64, u32)> = {
        let conn = STARBOARD_DB.lock().unwrap();
        conn.query_row(
            "SELECT channel_id, threshold FROM starboard_config WHERE guild_id = ?",
            [guild_id.get().cast_signed()],
            |row| {
                let channel_id: i64 = row.get(0)?;
                let threshold: i32 = row.get(1)?;
                Ok((channel_id.cast_unsigned(), threshold.cast_unsigned()))
            },
        )
        .ok()
    };

    let Some((starboard_channel_id, threshold)) = config else {
        return Ok(());
    };

    // Get the message
    let message = reaction
        .channel_id
        .message(ctx, reaction.message_id)
        .await?;

    // Count star reactions
    let star_count = message
        .reactions
        .iter()
        .find(|r| r.reaction_type == ReactionType::Unicode("⭐".to_string()))
        .map_or(0, |r| r.count);

    // Check if in starboard
    let starboard_entry: Option<(Option<i64>, i64)> = {
        let conn = STARBOARD_DB.lock().unwrap();
        conn.query_row(
            "SELECT starboard_message_id, posting FROM starred_messages WHERE message_id = ?",
            [reaction.message_id.get().cast_signed()],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .ok()
    };

    let Some((starboard_msg_id, posting)) = starboard_entry else {
        return Ok(());
    };

    if star_count < u64::from(threshold) {
        if let Some(starboard_msg_id) = starboard_msg_id {
            let deleted = poise::serenity_prelude::ChannelId::new(starboard_channel_id)
                .delete_message(
                    ctx,
                    poise::serenity_prelude::MessageId::new(starboard_msg_id.cast_unsigned()),
                )
                .await
                .is_ok();

            if deleted {
                let conn = STARBOARD_DB.lock().unwrap();
                conn.execute(
                    "DELETE FROM starred_messages WHERE message_id = ?",
                    [reaction.message_id.get().cast_signed()],
                )
                .ok();
            } else {
                let conn = STARBOARD_DB.lock().unwrap();
                conn.execute(
                    "UPDATE starred_messages SET star_count = ? WHERE message_id = ?",
                    [
                        star_count.cast_signed(),
                        reaction.message_id.get().cast_signed(),
                    ],
                )
                .ok();
            }
        } else if posting == 0 {
            let conn = STARBOARD_DB.lock().unwrap();
            conn.execute(
                "DELETE FROM starred_messages WHERE message_id = ?",
                [reaction.message_id.get().cast_signed()],
            )
            .ok();
        } else {
            let conn = STARBOARD_DB.lock().unwrap();
            conn.execute(
                "UPDATE starred_messages SET star_count = ? WHERE message_id = ?",
                [
                    star_count.cast_signed(),
                    reaction.message_id.get().cast_signed(),
                ],
            )
            .ok();
        }
    } else {
        // Update star count
        {
            let conn = STARBOARD_DB.lock().unwrap();
            conn.execute(
                "UPDATE starred_messages SET star_count = ? WHERE message_id = ?",
                [
                    star_count.cast_signed(),
                    reaction.message_id.get().cast_signed(),
                ],
            )
            .ok();
        }

        // Update the starboard message
        if let Some(starboard_msg_id) = starboard_msg_id
            && let Ok(mut starboard_msg) =
                poise::serenity_prelude::ChannelId::new(starboard_channel_id)
                    .message(
                        ctx,
                        poise::serenity_prelude::MessageId::new(starboard_msg_id.cast_unsigned()),
                    )
                    .await
        {
            let embed = create_star_embed(ctx, &message, star_count).await;
            starboard_msg
                .edit(ctx, EditMessage::new().embed(embed))
                .await
                .ok();
        }
    }

    Ok(())
}

async fn create_star_embed(
    ctx: &Context,
    message: &poise::serenity_prelude::Message,
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
        let reference_details = if let Some(reference_message) =
            message.referenced_message.as_deref()
        {
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
                    "**{}**\n{}\n[Jump to referenced message]({reference_url})",
                    reference_author_name, reference_preview
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
