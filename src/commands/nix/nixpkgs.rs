use color_eyre::eyre::{Result, eyre};
use poise::{CreateReply, serenity_prelude::CreateEmbed};

use crate::nixpkgs::packages;
use crate::types::Context;

/// Get information about a Nix package
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn nixpkgs(
    ctx: Context<'_>,
    #[description = "package name"] package: String,
) -> Result<()> {
    ctx.defer().await?;

    let pkg = tokio::task::spawn_blocking(move || packages::lookup(&package))
        .await??
        .ok_or_else(|| eyre!("Package not found"))?;

    let file = pkg.position.split(':').next().unwrap_or("unknown");
    let maintainers = if pkg.maintainers.is_empty() {
        "None".to_string()
    } else {
        pkg.maintainers
            .iter()
            .filter_map(|m| {
                let github = m.github.as_ref()?;
                Some(format!("[{}](https://github.com/{github})", m.name))
            })
            .collect::<Vec<String>>()
            .join(", ")
    };

    let embed = CreateEmbed::new()
        .title(format!("{} {}", pkg.pname, pkg.version))
        .url(format!(
            "https://github.com/nixos/nixpkgs/blob/master/{file}"
        ))
        .description(pkg.description)
        .field(
            "Homepage",
            pkg.homepage.unwrap_or_else(|| "N/A".to_string()),
            false,
        )
        .field("license", pkg.license, true)
        .field("insecure", pkg.insecure.to_string(), true)
        .field("unfree", pkg.unfree.to_string(), true)
        .field("broken", pkg.broken.to_string(), true)
        .field("maintainers", maintainers, false)
        .color(0x4D_6F_B7);

    ctx.send(CreateReply::default().embed(embed)).await?;
    Ok(())
}
