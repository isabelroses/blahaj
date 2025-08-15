use crate::types::Context;
use color_eyre::eyre::Result;
use poise::{serenity_prelude::CreateEmbed, CreateReply};
use std::process::Command;

#[derive(serde::Deserialize)]
struct PackageMeta {
    name: String,
    description: String,
    homepage: Option<String>,
    license: License,
    maintainers: Vec<Maintainers>,
    position: String,
    // broken: bool,
    insecure: bool,
    unfree: bool,
}

#[derive(serde::Deserialize)]
struct License {
    #[serde(rename = "spdxId")]
    spdx_id: String,
}

#[derive(serde::Deserialize)]
struct Maintainers {
    name: String,
    // email: Option<String>,
}

/// Track nixpkgs PRs
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn nixpkg(
    ctx: Context<'_>,
    #[description = "package name"] package: String,
) -> Result<()> {
    let pkg =
        Command::new("nix")
            .args([
                "eval",
                "--impure",
                "--json",
                "--expr",
                &format!(
                    "with import <nixpkgs> {{ config.allowUnfree = true; }}; pkgs.{package}.meta",
                ),
            ])
            .output()?;

    if !pkg.status.success() {
        ctx.say("Package not found or an error occurred.").await?;
        ctx.say(format!("Error: {}", String::from_utf8_lossy(&pkg.stderr)))
            .await?;
        return Ok(());
    }

    let pkg: PackageMeta = serde_json::from_slice(&pkg.stdout)?;

    let file = &pkg.position[51..];
    let fin_file = file
        .split_once(':')
        .map_or_else(|| file.to_string(), |(before, _)| before.to_string());

    let embed = CreateEmbed::new()
        .title(pkg.name)
        .url(format!(
            "https://github.com/nixos/nixpkgs/blob/master/{fin_file}"
        ))
        .description(pkg.description)
        .field(
            "Homepage",
            pkg.homepage.unwrap_or_else(|| "N/A".to_string()),
            false,
        )
        .field("license", pkg.license.spdx_id, true)
        .field("insecure", pkg.insecure.to_string(), true)
        .field("unfree", pkg.unfree.to_string(), true)
        .field(
            "maintainers",
            pkg.maintainers
                .iter()
                .map(|m| m.name.clone())
                .collect::<Vec<String>>()
                .join(", "),
            false,
        )
        .color(0x00DE_A586);

    ctx.send(CreateReply::default().embed(embed)).await?;

    Ok(())
}
