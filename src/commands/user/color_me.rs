use crate::types::Context;
use color_eyre::eyre::Result;
use poise::CreateReply;
use poise::serenity_prelude::{Colour, EditRole, RoleId, UserId};
use rusqlite::{Connection, params};
use std::sync::{LazyLock, Mutex};

static COLOR_DB: LazyLock<Mutex<Connection>> = LazyLock::new(|| {
    let db_path = std::env::var("COLOR_ROLES_DB").unwrap_or_else(|_| "color_roles.db".to_string());
    let conn = Connection::open(db_path).expect("Failed to open color roles database");

    conn.execute(
        "CREATE TABLE IF NOT EXISTS color_roles (
            user_id INTEGER PRIMARY KEY,
            guild_id INTEGER NOT NULL,
            role_id INTEGER NOT NULL,
            role_name TEXT NOT NULL
        )",
        [],
    )
    .expect("Failed to create color_roles table");

    Mutex::new(conn)
});

fn get_user_role(user_id: UserId, guild_id: u64) -> Option<(RoleId, String)> {
    let conn = COLOR_DB.lock().ok()?;
    let mut stmt = conn
        .prepare("SELECT role_id, role_name FROM color_roles WHERE user_id = ? AND guild_id = ?")
        .ok()?;

    stmt.query_row(params![user_id.get() as i64, guild_id as i64], |row| {
        let role_id: i64 = row.get(0)?;
        let role_name: String = row.get(1)?;
        Ok((RoleId::new(role_id as u64), role_name))
    })
    .ok()
}

fn save_user_role(user_id: UserId, guild_id: u64, role_id: RoleId, role_name: &str) -> Result<()> {
    let conn = COLOR_DB.lock().unwrap();
    conn.execute(
        "INSERT OR REPLACE INTO color_roles (user_id, guild_id, role_id, role_name) VALUES (?, ?, ?, ?)",
        params![user_id.get() as i64, guild_id as i64, role_id.get() as i64, role_name],
    )?;
    Ok(())
}

fn delete_user_role(user_id: UserId, guild_id: u64) -> Result<()> {
    let conn = COLOR_DB.lock().unwrap();
    conn.execute(
        "DELETE FROM color_roles WHERE user_id = ? AND guild_id = ?",
        params![user_id.get() as i64, guild_id as i64],
    )?;
    Ok(())
}

fn update_role_name(user_id: UserId, guild_id: u64, new_name: &str) -> Result<()> {
    let conn = COLOR_DB.lock().unwrap();
    conn.execute(
        "UPDATE color_roles SET role_name = ? WHERE user_id = ? AND guild_id = ?",
        params![new_name, user_id.get() as i64, guild_id as i64],
    )?;
    Ok(())
}

/// Change your display color or remove your color role.
#[poise::command(slash_command)]
pub async fn color_me(
    ctx: Context<'_>,
    #[description = "Hex color code (e.g., #FF5733 or FF5733)"] color: Option<String>,
    #[description = "Custom name for your role (optional)"] role_name: Option<String>,
) -> Result<()> {
    let Some(guild_id) = ctx.guild_id() else {
        ctx.send(
            CreateReply::default()
                .content("This command can only be used in a server.")
                .ephemeral(true),
        )
        .await?;
        return Ok(());
    };

    let member = guild_id.member(ctx.http(), ctx.author().id).await?;
    let username = &ctx.author().name;

    // check for existing role
    let db_role = get_user_role(ctx.author().id, guild_id.get());

    // they might not be in the db so lookup if there's a role with their username (previous behavior)
    let username_role = guild_id
        .roles(ctx.http())
        .await?
        .into_iter()
        .find(|(_, role)| role.name == *username);

    let existing_role = if let Some((role_id, stored_name)) = db_role {
        if guild_id.roles(ctx.http()).await?.contains_key(&role_id) {
            Some((role_id, stored_name))
        } else {
            // role was deleted from discord, clean up database
            delete_user_role(ctx.author().id, guild_id.get())?;
            None
        }
    } else if let Some((role_id, role)) = username_role {
        // migrate username-based role to database
        save_user_role(ctx.author().id, guild_id.get(), role_id, &role.name)?;
        Some((role_id, role.name.clone()))
    } else {
        None
    };

    match color {
        Some(color_str) => {
            // parse the hex color
            let color_str = color_str.trim_start_matches('#');
            let Ok(color_value) = u32::from_str_radix(color_str, 16) else {
                ctx.send(
                    CreateReply::default()
                        .content("Invalid hex color! Please provide a valid hex color code (e.g., #FF5733 or FF5733).")
                        .ephemeral(true),
                )
                .await?;
                return Ok(());
            };

            let colour_picked = Colour::new(color_value);
            let desired_role_name = role_name.as_deref().unwrap_or(username);

            if let Some((role_id, current_name)) = existing_role {
                guild_id
                    .edit_role(ctx.http(), role_id, EditRole::new().colour(colour_picked.0))
                    .await?;

                if let Some(new_name) = &role_name {
                    if new_name != &current_name {
                        guild_id
                            .edit_role(ctx.http(), role_id, EditRole::new().name(new_name))
                            .await?;
                        update_role_name(ctx.author().id, guild_id.get(), new_name)?;

                        ctx.send(
                            CreateReply::default()
                                .content(format!("Updated your color to `#{color_str}` and renamed your role to `{new_name}`! <3"))
                                .ephemeral(true),
                        )
                        .await?;
                    } else {
                        ctx.send(
                            CreateReply::default()
                                .content(format!("Updated your color to `#{color_str}`! <3"))
                                .ephemeral(true),
                        )
                        .await?;
                    }
                } else {
                    ctx.send(
                        CreateReply::default()
                            .content(format!("Updated your color to `#{color_str}`! <3"))
                            .ephemeral(true),
                    )
                    .await?;
                }
            } else {
                let new_role = guild_id
                    .create_role(
                        ctx.http(),
                        EditRole::new()
                            .name(desired_role_name)
                            .colour(colour_picked.0)
                            .hoist(false)
                            .mentionable(false),
                    )
                    .await?;

                member.add_role(ctx.http(), new_role.id).await?;
                save_user_role(
                    ctx.author().id,
                    guild_id.get(),
                    new_role.id,
                    desired_role_name,
                )?;

                ctx.send(
                    CreateReply::default()
                        .content(format!(
                            "Created a new role `{desired_role_name}` and set your color to `#{color_str}`! <3"
                        ))
                        .ephemeral(true),
                )
                .await?;
            }
        }
        None => {
            if let Some(new_name) = role_name {
                if let Some((role_id, current_name)) = existing_role {
                    if new_name != current_name {
                        guild_id
                            .edit_role(ctx.http(), role_id, EditRole::new().name(&new_name))
                            .await?;
                        update_role_name(ctx.author().id, guild_id.get(), &new_name)?;

                        ctx.send(
                            CreateReply::default()
                                .content(format!("Renamed your role to `{new_name}`! <3"))
                                .ephemeral(true),
                        )
                        .await?;
                    } else {
                        ctx.send(
                            CreateReply::default()
                                .content("Your role already has that name!")
                                .ephemeral(true),
                        )
                        .await?;
                    }
                } else {
                    ctx.send(
                        CreateReply::default()
                            .content("You don't have a color role yet! Use `/color_me` with a color to create one.")
                            .ephemeral(true),
                    )
                    .await?;
                }
            } else {
                match existing_role {
                    Some((role_id, _)) => {
                        member.remove_role(ctx.http(), role_id).await?;
                        guild_id.delete_role(ctx.http(), role_id).await?;
                        delete_user_role(ctx.author().id, guild_id.get())?;

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
    }

    Ok(())
}
