use poise::CreateReply;

fn no_mentions() -> poise::serenity_prelude::CreateAllowedMentions {
    poise::serenity_prelude::CreateAllowedMentions::new()
        .everyone(false)
        .all_users(false)
        .all_roles(false)
        .replied_user(false)
        .empty_users()
        .empty_roles()
}

pub fn safe_reply() -> CreateReply {
    CreateReply::default().allowed_mentions(no_mentions())
}
