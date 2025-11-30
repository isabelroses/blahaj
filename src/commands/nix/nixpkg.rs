use std::fs::{self, File};
use std::io::BufReader;

use crate::types::Context;
use color_eyre::eyre::Result;
use poise::{serenity_prelude::CreateEmbed, CreateReply};

#[derive(serde::Deserialize)]
struct Package {
    pname: String,
    version: String,
    meta: PackageMeta,
}

#[derive(serde::Deserialize)]
struct PackageMeta {
    description: String,
    homepage: Option<String>,
    license: License,
    maintainers: Vec<Maintainers>,
    position: String,
    broken: bool,
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
    github: String,
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
    ctx.defer().await?;

    let nixpkgs_json =
        std::env::var("NIXPKGS_JSON").expect("NIXPKGS_JSON environment variable must be set");

    let file = File::open(nixpkgs_json)?;
    let reader = BufReader::new(file);
    let mut pkgs: serde_json::Value = serde_json::from_reader(reader)?;
    let pkg: Package = serde_json::from_value(pkgs["packages"][&package].take())?;

    let file = pkg.meta.position.split(':').next().unwrap_or("unknown");

    let embed = CreateEmbed::new()
        .title(format!("{} {}", pkg.pname, pkg.version))
        .url(format!(
            "https://github.com/nixos/nixpkgs/blob/master/{file}"
        ))
        .description(pkg.meta.description)
        .field(
            "Homepage",
            pkg.meta.homepage.unwrap_or_else(|| "N/A".to_string()),
            false,
        )
        .field("license", pkg.meta.license.spdx_id, true)
        .field("insecure", pkg.meta.insecure.to_string(), true)
        .field("unfree", pkg.meta.unfree.to_string(), true)
        .field("broken", pkg.meta.broken.to_string(), true)
        .field(
            "maintainers",
            pkg.meta
                .maintainers
                .iter()
                .map(|m| format!("[{}](https://github.com/{})", m.name, m.github))
                .collect::<Vec<String>>()
                .join(", "),
            false,
        )
        .color(0x00DE_A586);

    ctx.send(CreateReply::default().embed(embed)).await?;

    Ok(())
}
