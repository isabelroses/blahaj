use color_eyre::eyre::{eyre, Result};
use poise::serenity_prelude::{Context, FullEvent};
use reqwest::Client;
use serenity::all::{ChannelId, Member, PartialMember, RoleId, UserId};

pub async fn handle(ctx: &Context, event: &FullEvent, _client: &Client) -> Result<()> {
    if let FullEvent::GuildMemberAddition { new_member } = event {
        warn_onboarding(ctx, &new_member.user.id).await?;
    } else if let FullEvent::GuildMemberUpdate {
        old_if_available: _,
        new,
        event: _,
    } = event
    {
        if let Some(member) = new {
            // check if the user does not have the kitten role
            if !member.roles.iter().any(|role| *role == 1249814690486423612) {
                // check if the user has the pronouns role
                if member
                    .roles
                    .iter()
                    .filter(|role| is_pronouns_role(**role))
                    .count()
                    > 0
                {
                    add_kitten_role(ctx, member).await?;
                }
            }
        }
    } else if let FullEvent::Message { new_message } = event {
        let user = &new_message.author;
        let partial_member: PartialMember = user
            .member
            .clone()
            .ok_or(eyre!("Failed to get member"))?
            .as_mut()
            .to_owned();
        let member = Member::from(partial_member);
        // check if the user has the kitten role
        if member.roles.iter().any(|role| *role == 1249814690486423612) {
            // check if the user does not have pronouns selected
            if member
                .roles
                .iter()
                .filter(|role| is_pronouns_role(**role))
                .count()
                == 0
            {
                // delete the message
                new_message
                    .delete(ctx)
                    .await
                    .map_err(|e| eyre!("Failed to delete message: {}", e))?;

                // send a message
                warn_onboarding(ctx, &user.id).await?;

                // remove the kitten role

                member
                    .remove_role(ctx, 1249814690486423612)
                    .await
                    .map_err(|e| eyre!("Failed to remove role: {}", e))?;
            }
        }
    }
    Ok(())
}

async fn warn_onboarding(ctx: &Context, user_id: &UserId) -> Result<(), color_eyre::eyre::Error> {
    ChannelId::new(1095084404168200302)
        .say(
            ctx,
            format!(
                "Welcome to the server, <@{user_id}>!\nPlease select your roles and pronouns from onboarding to get started."
            ),
        )
        .await?;
    Ok(())
}

async fn add_kitten_role(ctx: &Context, member: &Member) -> Result<()> {
    member
        .add_role(ctx, RoleId::new(1249814690486423612))
        .await
        .map_err(|e| eyre!("Failed to add role: {}", e))
}

fn is_pronouns_role(role: RoleId) -> bool {
    role == 1095084950107209728 // she/her
        || role == 1095085000241709217 // he/him
        || role == 1095085169381232770 // they/them
        || role == 1095085419265269922 // ask for pronouns
}
