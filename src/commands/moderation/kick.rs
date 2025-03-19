use crate::types::Context;
use color_eyre::Result;
use poise::serenity_prelude::all::User;

#[poise::command(slash_command, guild_only, required_permissions = "KICK_MEMBERS")]
pub async fn kick(ctx: Context<'_>, user: User, reason: Option<String>) -> Result<()> {
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
        .kick_members()
    {
        ctx.say("Bot missing permission: ``Kick Members``").await?;
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
        .kick_with_reason(ctx, &reason.unwrap_or("No reason provided.".to_string()))
        .await?;

    Ok(())
}
