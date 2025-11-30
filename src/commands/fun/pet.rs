use crate::types::Context;
use color_eyre::eyre::Result;
use poise::{CreateReply, serenity_prelude::all::User};
use serenity::all::CreateAttachment;

#[derive(serde::Serialize, serde::Deserialize)]
struct PetPet {
    image: String,
}

/// Displays your or another user's info
#[poise::command(slash_command)]
pub async fn pet(
    ctx: Context<'_>,
    #[description = "Selected user"] user: Option<User>,
) -> Result<()> {
    let user = user.as_ref().unwrap_or_else(|| ctx.author());

    let pet = PetPet {
        image: user
            .avatar_url()
            .expect("avatar failed")
            .replace("webp", "png"),
    };

    let res = ctx
        .data()
        .client
        .get("https://memeado.vercel.app/api/petpet")
        .json(&pet)
        .send()
        .await?
        .bytes()
        .await?;

    let attachment = CreateAttachment::bytes(res, "petpet.gif");

    let reply = CreateReply::default().attachment(attachment);

    ctx.send(reply).await?;
    Ok(())
}
