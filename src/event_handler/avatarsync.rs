use color_eyre::eyre::Result;
use poise::serenity_prelude::{Context, FullEvent, User};

use crate::avatarsync;
use crate::types::Data;

pub async fn handle(ctx: &Context, event: &FullEvent, data: &Data) -> Result<()> {
    match event {
        FullEvent::GuildMemberUpdate {
            old_if_available,
            event,
            ..
        } => {
            if old_if_available
                .as_ref()
                .is_some_and(|old| old.user.avatar == event.user.avatar)
            {
                return Ok(());
            }
            avatarsync::refresh(&ctx.http, &data.client, &event.user, event.guild_id, false)
                .await?;
        }
        FullEvent::UserUpdate { old_data, new } => {
            if old_data
                .as_ref()
                .is_some_and(|old| old.avatar == new.avatar)
            {
                return Ok(());
            }
            avatarsync::refresh_everywhere(&ctx.http, &data.client, &User::from(new.clone()))
                .await?;
        }
        _ => {}
    }

    Ok(())
}
