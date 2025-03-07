use crate::Context;
use color_eyre::eyre::Result;
use poise::serenity_prelude::{OnlineStatus, UserId};
use rand::Rng;

/// Rolls dice based on given # of sides
#[poise::command(slash_command)]
pub async fn roll(
    ctx: Context<'_>,
    #[description = "# of sides"] sides: Option<u32>,
) -> Result<()> {
    let sides = sides.unwrap_or(6);
    let roll = rand::rng().random_range(1..=sides);
    ctx.say(format!("You rolled a **{roll}**")).await?;
    Ok(())
}

/// Select a random person to win a raffle
#[poise::command(slash_command)]
pub async fn raffle(ctx: Context<'_>) -> Result<()> {
    let mut memeberid: UserId = UserId::new(1);

    let members = ctx
        .guild_id()
        .unwrap()
        .members(&ctx.http(), None, None)
        .await?;

    let mut find_member = false;
    while !find_member {
        let selected = rand::rng().random_range(1..=members.len());
        let memeber = &members[selected].user;
        memeberid = memeber.id;

        if let Some(presence) = ctx.guild().unwrap().presences.get(&memeberid) {
            find_member = presence.status == OnlineStatus::Online
                || presence.status == OnlineStatus::Idle
                    && !memeber.bot
                    && memeberid != ctx.author().id;
        }
    }

    if Some(memeberid).is_some() && memeberid != UserId::new(1) {
        ctx.say(format!("<@{memeberid}> has won the raffle"))
            .await?;
    }

    Ok(())
}
