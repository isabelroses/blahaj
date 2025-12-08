use crate::types::Context;
use color_eyre::Result;
use poise::serenity_prelude::all::{EditMember, Timestamp, User};
use regex::Regex;

#[poise::command(slash_command, guild_only, required_permissions = "MODERATE_MEMBERS")]
pub async fn timeout(
    ctx: Context<'_>,
    user: User,
    #[description = "how long the time out is for. (5s, 2m, 12h, 3d, 2w)"] duration: String,
    reason: Option<String>,
) -> Result<()> {
    let guild = ctx
        .serenity_context()
        .http
        .get_guild(ctx.guild_id().unwrap())
        .await?;

    let timeout_max_value = 2419200;

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

    if guild
        .user_permissions_in(
            &ctx.guild_channel().await.unwrap(),
            &guild.member(ctx, user.id).await.unwrap(),
        )
        .administrator()
    {
        ctx.say("Target user has permission: ``Administrator``")
            .await?;
        return Ok(());
    }

    if !guild
        .user_permissions_in(
            &ctx.guild_channel().await.unwrap(),
            &guild.member(ctx, ctx.framework().bot_id).await.unwrap(),
        )
        .moderate_members()
    {
        ctx.say("Bot missing permission: ``Moderate Members``")
            .await?;
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

    let Some(seconds) = parse_duration(&duration) else {
        ctx.say("Invalid duration! (Valid durations: 5s, 2m, 12h, 3d, 2w)")
            .await?;
        return Ok(());
    };

    if seconds > timeout_max_value {
        ctx.say("Duration too long! (Max duration: 28d)").await?;
        return Ok(());
    }

    let seconds = i64::try_from(seconds);
    if seconds.is_err() {
        ctx.say("Duration value is too large!").await?;
        return Ok(());
    }

    // safer unwrap then cloudflare lol
    let timeout_until =
        Timestamp::from_unix_timestamp(Timestamp::now().unix_timestamp() + seconds.unwrap())?;

    guild
        .member(ctx, user.id)
        .await?
        .edit(
            ctx,
            EditMember::new()
                .disable_communication_until(timeout_until.to_string())
                .audit_log_reason(&reason.unwrap_or("No reason provided.".to_string())),
        )
        .await?;

    ctx.say(format!("Timed out <@{}> for {}!", user.id, duration))
        .await?;

    Ok(())
}

fn parse_duration(input: &str) -> Option<u64> {
    let re = Regex::new(r"(\d+)([smhdw])").unwrap();

    if let Some(caps) = re.captures(input) {
        let value: u64 = caps[1].parse().ok()?;
        let unit = &caps[2];

        let seconds = match unit {
            "s" => value,
            "m" => value * 60,
            "h" => value * 3600,
            "d" => value * 86400,
            "w" => value * 604800,
            _ => return None,
        };

        Some(seconds)
    } else {
        None
    }
}
