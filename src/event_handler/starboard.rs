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

    // Check if already starred
    let already_starred: bool = {
        let conn = STARBOARD_DB.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM starred_messages WHERE message_id = ?",
            [reaction.message_id.get().cast_signed()],
            |row| {
                let count: i32 = row.get(0)?;
                Ok(count > 0)
            },
        )
        .unwrap_or(false)
    };

    if already_starred {
        // Update the star count
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

        // Update the starboard message if it exists
        let starboard_msg_id: Option<i64> = {
            let conn = STARBOARD_DB.lock().unwrap();
            conn.query_row(
                "SELECT starboard_message_id FROM starred_messages WHERE message_id = ?",
                [reaction.message_id.get().cast_signed()],
                |row| row.get::<_, i64>(0),
            )
            .ok()
        };

        if let Some(starboard_msg_id) = starboard_msg_id
            && let Ok(mut starboard_msg) =
                poise::serenity_prelude::ChannelId::new(starboard_channel_id)
                    .message(
                        ctx,
                        poise::serenity_prelude::MessageId::new(starboard_msg_id.cast_unsigned()),
                    )
                    .await
        {
            let embed = create_star_embed(&message, star_count);
            starboard_msg
                .edit(ctx, EditMessage::new().embed(embed))
                .await
                .ok();
        }

        return Ok(());
    }

    // Create starboard message
    let starboard_channel = poise::serenity_prelude::ChannelId::new(starboard_channel_id);
    let embed = create_star_embed(&message, star_count);

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
        // Save to database
        let conn = STARBOARD_DB.lock().unwrap();
        conn.execute(
            "INSERT INTO starred_messages (message_id, guild_id, channel_id, starboard_message_id, star_count) VALUES (?, ?, ?, ?, ?)",
            [
                reaction.message_id.get().cast_signed(),
                guild_id.get().cast_signed(),
                reaction.channel_id.get().cast_signed(),
                starboard_msg.id.get().cast_signed(),
                star_count.cast_signed(),
            ],
        ).ok();
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
    let starboard_msg_id: Option<i64> = {
        let conn = STARBOARD_DB.lock().unwrap();
        conn.query_row(
            "SELECT starboard_message_id FROM starred_messages WHERE message_id = ?",
            [reaction.message_id.get().cast_signed()],
            |row| row.get(0),
        )
        .ok()
    };

    let Some(starboard_msg_id) = starboard_msg_id else {
        return Ok(());
    };

    if star_count < u64::from(threshold) {
        // Remove from starboard
        poise::serenity_prelude::ChannelId::new(starboard_channel_id)
            .delete_message(
                ctx,
                poise::serenity_prelude::MessageId::new(starboard_msg_id.cast_unsigned()),
            )
            .await
            .ok();

        let conn = STARBOARD_DB.lock().unwrap();
        conn.execute(
            "DELETE FROM starred_messages WHERE message_id = ?",
            [reaction.message_id.get().cast_signed()],
        )
        .ok();
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
        if let Ok(mut starboard_msg) = poise::serenity_prelude::ChannelId::new(starboard_channel_id)
            .message(
                ctx,
                poise::serenity_prelude::MessageId::new(starboard_msg_id.cast_unsigned()),
            )
            .await
        {
            let embed = create_star_embed(&message, star_count);
            starboard_msg
                .edit(ctx, EditMessage::new().embed(embed))
                .await
                .ok();
        }
    }

    Ok(())
}

fn create_star_embed(
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

    if let Some(first_attachment) = message.attachments.first() {
        embed = embed.image(&first_attachment.url);
    }

    embed
}
