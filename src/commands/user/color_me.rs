use crate::types::Context;
use color_eyre::eyre::Result;
use poise::serenity_prelude::{Colour, EditRole};
use poise::CreateReply;

/// Change your display color or remove your color role.
#[poise::command(slash_command)]
pub async fn color_me(
    ctx: Context<'_>,
    #[description = "Hex color code (e.g., #FF5733 or FF5733)"] color: Option<String>,
) -> Result<()> {
    let guild_id = match ctx.guild_id() {
        Some(id) => id,
        None => {
            ctx.send(
                CreateReply::default()
                    .content("This command can only be used in a server.")
                    .ephemeral(true),
            )
            .await?;
            return Ok(());
        }
    };

    let member = guild_id.member(ctx.http(), ctx.author().id).await?;
    let username = &ctx.author().name;

    // lookup if user already has a role with their username
    let existing_role = guild_id
        .roles(ctx.http())
        .await?
        .into_iter()
        .find(|(_, role)| role.name == *username);

    match color {
        Some(color_str) => {
            // parse the hex color
            let color_str = color_str.trim_start_matches('#');
            let color_value = match u32::from_str_radix(color_str, 16) {
                Ok(val) => val,
                Err(_) => {
                    ctx.send(
                        CreateReply::default()
                            .content("Invalid hex color! Please provide a valid hex color code (e.g., #FF5733 or FF5733).")
                            .ephemeral(true),
                    )
                    .await?;
                    return Ok(());
                }
            };

            let colour = Colour::new(color_value);

            match existing_role {
                Some((role_id, _)) => {
                    // update existing role
                    guild_id
                        .edit_role(ctx.http(), role_id, EditRole::new().colour(colour.0))
                        .await?;
                    ctx.send(
                        CreateReply::default()
                            .content(format!("Updated your color to `#{color_str}`! <3"))
                            .ephemeral(true),
                    )
                    .await?;
                }
                None => {
                    // or else create new role
                    let new_role = guild_id
                        .create_role(
                            ctx.http(),
                            EditRole::new()
                                .name(username)
                                .colour(colour.0)
                                .hoist(false)
                                .mentionable(false),
                        )
                        .await?;

                    member.add_role(ctx.http(), new_role.id).await?;

                    ctx.send(
                        CreateReply::default()
                            .content(format!(
                                "Created a new role and set your color to `#{color_str}`! <3"
                            ))
                            .ephemeral(true),
                    )
                    .await?;
                }
            }
        }
        None => {
            // remove the role if it exists
            match existing_role {
                Some((role_id, _)) => {
                    member.remove_role(ctx.http(), role_id).await?;
                    guild_id.delete_role(ctx.http(), role_id).await?;

                    ctx.send(
                        CreateReply::default()
                            .content("Removed your color role.")
                            .ephemeral(true),
                    )
                    .await?;
                }
                None => {
                    ctx.send(
                        CreateReply::default()
                            .content("You don't have a color role to remove.")
                            .ephemeral(true),
                    )
                    .await?;
                }
            }
        }
    }

    Ok(())
}
