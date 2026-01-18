use color_eyre::eyre::Result;
use poise::serenity_prelude::{Context, FullEvent, ReactionType, Colour, EditMessage};
use rusqlite::Connection;
use std::sync::{LazyLock, Mutex};

use crate::types::Data;

static STARBOARD_DB: LazyLock<Mutex<Connection>> = LazyLock::new(|| {
    let db_path = crate::utils::get_data_dir().join("starboard.db");
    let conn = Connection::open(db_path).expect("Failed to open starboard database");

    conn.execute(
        "CREATE TABLE IF NOT EXISTS starred_messages (
            message_id INTEGER PRIMARY KEY,
            guild_id INTEGER NOT NULL,
            channel_id INTEGER NOT NULL,
            starboard_message_id INTEGER,
            star_count INTEGER NOT NULL DEFAULT 1,
            UNIQUE(message_id)
        )",
        [],
    )
    .expect("Failed to create starred_messages table");

    conn.execute(
        "CREATE TABLE IF NOT EXISTS starboard_config (
            guild_id INTEGER PRIMARY KEY,
            channel_id INTEGER NOT NULL,
            threshold INTEGER NOT NULL DEFAULT 3
        )",
        [],
    )
    .expect("Failed to create starboard_config table");

    Mutex::new(conn)
});

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
    let config: Option<(u64, i32)> = {
        let conn = STARBOARD_DB.lock().unwrap();
        conn.query_row(
            "SELECT channel_id, threshold FROM starboard_config WHERE guild_id = ?",
            [guild_id.get() as i64],
            |row| {
                let channel_id: i64 = row.get(0)?;
                let threshold: i32 = row.get(1)?;
                Ok((channel_id as u64, threshold))
            },
        )
        .ok()
    };

    let Some((starboard_channel_id, threshold)) = config else {
        return Ok(());
    };

    // Get the message
    let message = reaction.channel_id.message(ctx, reaction.message_id).await?;

    // Count star reactions
    let star_count = message
        .reactions
        .iter()
        .find(|r| r.reaction_type == ReactionType::Unicode("⭐".to_string()))
        .map_or(0, |r| r.count);

    if star_count < threshold as u64 {
        return Ok(());
    }

    // Check if already starred
    let already_starred: bool = {
        let conn = STARBOARD_DB.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM starred_messages WHERE message_id = ?",
            [reaction.message_id.get() as i64],
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
                [star_count as i64, reaction.message_id.get() as i64],
            )
            .ok();
        }

        // Update the starboard message if it exists
        let starboard_msg_id: Option<i64> = {
            let conn = STARBOARD_DB.lock().unwrap();
            conn.query_row(
                "SELECT starboard_message_id FROM starred_messages WHERE message_id = ?",
                [reaction.message_id.get() as i64],
                |row| row.get::<_, i64>(0),
            )
            .ok()
        };

        if let Some(starboard_msg_id) = starboard_msg_id
            && let Ok(mut starboard_msg) = poise::serenity_prelude::ChannelId::new(starboard_channel_id)
                .message(ctx, poise::serenity_prelude::MessageId::new(starboard_msg_id as u64))
                .await
            {
                let embed = create_star_embed(&message, star_count as i32);
                starboard_msg.edit(ctx, EditMessage::new().embed(embed)).await.ok();
            }

        return Ok(());
    }

    // Create starboard message
    let starboard_channel = poise::serenity_prelude::ChannelId::new(starboard_channel_id);
    let embed = create_star_embed(&message, star_count as i32);

    if let Ok(starboard_msg) = starboard_channel
        .send_message(
            ctx,
            poise::serenity_prelude::CreateMessage::new()
                .embed(embed)
                .content(format!("<#{}>", reaction.channel_id)),
        )
        .await
    {
        // Save to database
        let conn = STARBOARD_DB.lock().unwrap();
        conn.execute(
            "INSERT INTO starred_messages (message_id, guild_id, channel_id, starboard_message_id, star_count) VALUES (?, ?, ?, ?, ?)",
            [
                reaction.message_id.get() as i64,
                guild_id.get() as i64,
                reaction.channel_id.get() as i64,
                starboard_msg.id.get() as i64,
                star_count as i64,
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
    let config: Option<(u64, i32)> = {
        let conn = STARBOARD_DB.lock().unwrap();
        conn.query_row(
            "SELECT channel_id, threshold FROM starboard_config WHERE guild_id = ?",
            [guild_id.get() as i64],
            |row| {
                let channel_id: i64 = row.get(0)?;
                let threshold: i32 = row.get(1)?;
                Ok((channel_id as u64, threshold))
            },
        )
        .ok()
    };

    let Some((starboard_channel_id, threshold)) = config else {
        return Ok(());
    };

    // Get the message
    let message = reaction.channel_id.message(ctx, reaction.message_id).await?;

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
            [reaction.message_id.get() as i64],
            |row| row.get(0),
        )
        .ok()
    };

    let Some(starboard_msg_id) = starboard_msg_id else {
        return Ok(());
    };

    if star_count < threshold as u64 {
        // Remove from starboard
        poise::serenity_prelude::ChannelId::new(starboard_channel_id)
            .delete_message(ctx, poise::serenity_prelude::MessageId::new(starboard_msg_id as u64))
            .await
            .ok();

        let conn = STARBOARD_DB.lock().unwrap();
        conn.execute(
            "DELETE FROM starred_messages WHERE message_id = ?",
            [reaction.message_id.get() as i64],
        )
        .ok();
    } else {
        // Update star count
        {
            let conn = STARBOARD_DB.lock().unwrap();
            conn.execute(
                "UPDATE starred_messages SET star_count = ? WHERE message_id = ?",
                [star_count as i64, reaction.message_id.get() as i64],
            )
            .ok();
        }

        // Update the starboard message
        if let Ok(mut starboard_msg) = poise::serenity_prelude::ChannelId::new(starboard_channel_id)
            .message(ctx, poise::serenity_prelude::MessageId::new(starboard_msg_id as u64))
            .await
        {
            let embed = create_star_embed(&message, star_count as i32);
            starboard_msg.edit(ctx, EditMessage::new().embed(embed)).await.ok();
        }
    }

    Ok(())
}

fn create_star_embed(
    message: &poise::serenity_prelude::Message,
    star_count: i32,
) -> poise::serenity_prelude::CreateEmbed {
    let mut embed = poise::serenity_prelude::CreateEmbed::default()
        .author(
            poise::serenity_prelude::CreateEmbedAuthor::new(&message.author.name)
                .icon_url(message.author.face()),
        )
        .description(&message.content)
        .footer(
            poise::serenity_prelude::CreateEmbedFooter::new(format!("⭐ {star_count}")),
        )
        .colour(Colour::GOLD)
        .timestamp(message.timestamp);

    if let Some(first_attachment) = message.attachments.first() {
        embed = embed.image(&first_attachment.url);
    }

    embed
}
