use crate::Result;
use poise::serenity_prelude::all::User;

use crate::types::Context;

#[poise::command(slash_command, guild_only, required_permissions = "BAN_MEMBERS")]
pub async fn ban(
    ctx: Context<'_>,
    user: User,
    delete_messages_day_count: Option<u8>,
    reason: Option<String>,
) -> Result<()> {
    let guild = ctx
        .serenity_context()
        .http
        .get_guild(ctx.guild_id().unwrap())
        .await?;

    let user_member = guild.member(ctx, user.id).await.unwrap();
    let bot_member = guild.member(ctx, ctx.framework().bot_id).await.unwrap();
    let author_member = guild.member(ctx, ctx.author()).await.unwrap();

    let user_highest_role = user_member
        .roles
        .iter()
        .filter_map(|role| guild.roles.get(role))
        .max_by_key(|role| role.position);

    let author_highest_role = author_member
        .roles
        .iter()
        .filter_map(|role| guild.roles.get(role))
        .max_by_key(|role| role.position);

    let bot_highest_role = bot_member
        .roles
        .iter()
        .filter_map(|role| guild.roles.get(role))
        .max_by_key(|role| role.position);

    if !guild
        .user_permissions_in(
            &ctx.guild_channel().await.unwrap(),
            &guild.member(ctx, ctx.framework().bot_id).await.unwrap(),
        )
        .ban_members()
    {
        ctx.say("Bot missing permission: ``Ban Members``").await?;
        return Ok(());
    }

    if author_highest_role < user_highest_role {
        ctx.say("User has higher role then you.").await?;
        return Ok(());
    }

    if bot_highest_role < user_highest_role {
        ctx.say("User has higher role then bot!").await?;
        return Ok(());
    }

    guild
        .member(ctx, user.id)
        .await?
        .ban_with_reason(
            ctx,
            delete_messages_day_count.unwrap_or(0),
            &reason.unwrap_or("No reason provided.".to_string()),
        )
        .await?;

    Ok(())
}
