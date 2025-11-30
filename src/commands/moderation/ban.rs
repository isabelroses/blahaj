use crate::Result;
use poise::serenity_prelude::RoleId;
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

    let bot_member = guild.member(ctx, ctx.framework().bot_id).await.unwrap();
    let author_member = guild.member(ctx, ctx.author()).await.unwrap();

    // Only check perms if user is in guild
    if let Ok(user_member) = guild.member(ctx, &user).await {
        let get_role_pos = |roles: &[RoleId]| {
            roles
                .iter()
                .filter_map(|role| guild.roles.get(role))
                .max_by_key(|role| role.position)
        };

        let user_highest_role = get_role_pos(&user_member.roles);
        let author_highest_role = get_role_pos(&author_member.roles);
        let bot_highest_role = get_role_pos(&bot_member.roles);

        if author_highest_role < user_highest_role {
            ctx.say("User has higher role than you.").await?;
            return Ok(());
        }

        if bot_highest_role < user_highest_role {
            ctx.say("User has higher role than bot!").await?;
            return Ok(());
        }
    }

    if !guild
        .user_permissions_in(&ctx.guild_channel().await.unwrap(), &bot_member)
        .ban_members()
    {
        ctx.say("Bot missing permission: ``Ban Members``").await?;
        return Ok(());
    }

    guild
        .ban_with_reason(
            ctx,
            &user,
            delete_messages_day_count.unwrap_or(0),
            &reason.unwrap_or("No reason provided.".to_string()),
        )
        .await?;

    ctx.say(format!("Banned user {user}.")).await?;

    Ok(())
}
