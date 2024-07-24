use color_eyre::eyre::Result;
use poise::{serenity_prelude::CreateEmbed, CreateReply};
use reqwest::StatusCode;
use serde::Deserialize;
use serenity::all::{CreateEmbedAuthor, CreateEmbedFooter, Timestamp};

use crate::Context;

const CRATES_API_URL: &str = "https://crates.io/api/v1";
const CRATES_PUBLIC_URL: &str = "https://lib.rs";

#[derive(Debug, Deserialize)]
struct Error {
    errors: Vec<ErrorItem>,
}
#[derive(Debug, Deserialize)]
struct ErrorItem {
    detail: String,
}

#[derive(Debug, Deserialize)]
struct _Crate {
    #[serde(rename = "crate")]
    c: Crate,
    versions: Vec<Version>,
}

#[derive(Debug, Deserialize)]
struct Crate {
    categories: Vec<String>,
    homepage: Option<String>,
    description: String,
    max_stable_version: String,
}

#[derive(Debug, Deserialize)]
struct Version {
    updated_at: String,
    license: Option<String>,
    published_by: Option<Publisher>,
    yanked: bool,
}

#[derive(Debug, Deserialize)]
struct Publisher {
    avatar: String,
    login: String,
    url: String,
}

#[poise::command(slash_command)]
pub async fn crates(ctx: Context<'_>, #[description = "crate"] crt: String) -> Result<()> {
    ctx.defer().await?;

    let client = &ctx.data().client;

    let url = format!("{CRATES_API_URL}/crates/{crt}");
    let resp = client.get(&url).send().await?;

    let mut embed = CreateEmbed::default()
        .url(format!("{CRATES_PUBLIC_URL}/{crt}"))
        .title(format!("Rust Crate `{crt}` Info"))
        .color(0xDEA586);

    match resp.status() {
        StatusCode::OK => {
            let resp = resp.json::<_Crate>().await?;
            let version = &resp.versions[0];
            let yanked = match version.yanked {
                true => " ❌",
                false => "",
            };
            let footer = CreateEmbedFooter::new(format!(
                "Latest Stable:{yanked} {crt} {}",
                resp.c.max_stable_version
            ));

            embed = embed
                .description(resp.c.description)
                .timestamp(Timestamp::parse(&version.updated_at)?)
                .field(
                    "Categories",
                    resp.c
                        .categories
                        .iter()
                        .map(|category| format!("[**{category}**]({CRATES_PUBLIC_URL}/{category})"))
                        .collect::<Vec<String>>()
                        .join(", "),
                    false,
                )
                .field(
                    "Homepage",
                    resp.c.homepage.unwrap_or_else(|| String::from("N/A")),
                    true,
                )
                .field(
                    "License",
                    version
                        .license
                        .as_ref()
                        .map(|s| s.as_str())
                        .unwrap_or_else(|| "N/A"),
                    true,
                )
                .footer(footer);

            if let Some(publisher) = &version.published_by {
                let author = CreateEmbedAuthor::new(&publisher.login)
                    .url(&publisher.url)
                    .icon_url(&publisher.avatar);

                embed = embed.author(author).thumbnail(&publisher.avatar);
            }
        }
        _ => {
            let err = resp.json::<Error>().await;
            let err = err?;
            let err = err
                .errors
                .into_iter()
                .map(|d| ("❌ Error", format!("```{}```", d.detail), false));
            embed = embed.fields(err)
        }
    }

    ctx.send(CreateReply::default().embed(embed)).await?;

    Ok(())
}
