use color_eyre::eyre::Result;
use poise::{CreateReply, serenity_prelude::CreateEmbed};
use reqwest::StatusCode;
use serde::Deserialize;
use serenity::all::{CreateEmbedAuthor, CreateEmbedFooter, Timestamp};

use crate::types::Context;

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
    repository: Option<String>,
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

#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn crates(ctx: Context<'_>, #[description = "crate"] crt: String) -> Result<()> {
    ctx.defer().await?;

    let client = &ctx.data().client;

    let url = format!("{CRATES_API_URL}/crates/{crt}");
    let resp = client.get(&url).send().await?;

    let mut embed = CreateEmbed::default()
        .url(format!("{CRATES_PUBLIC_URL}/{crt}"))
        .title(format!("Rust Crate `{crt}` Info"))
        .color(0x00DE_A586);

    if resp.status() == StatusCode::OK {
        let resp = resp.json::<_Crate>().await?;
        let version = &resp.versions[0];
        let yanked = if version.yanked { " ❌" } else { "" };
        let footer = CreateEmbedFooter::new(format!(
            "Latest Stable:{yanked} {crt} {}",
            resp.c.max_stable_version
        ));

        if !resp.c.categories.is_empty() {
            embed = embed.field(
                "Categories",
                resp.c
                    .categories
                    .iter()
                    .map(|category| format!("[**{category}**]({CRATES_PUBLIC_URL}/{category})"))
                    .collect::<Vec<String>>()
                    .join(", "),
                false,
            );
        }

        embed = match (resp.c.homepage, resp.c.repository) {
            (Some(homepage), _) => embed.field("Homepage", homepage, true),
            (None, Some(repository)) => embed.field("Repository", repository, true),
            (None, None) => embed.field("Homepage", "N/A", true),
        };

        embed = embed
            .description(resp.c.description)
            .timestamp(Timestamp::parse(&version.updated_at)?)
            .field(
                "License",
                version
                    .license
                    .as_ref()
                    .map_or_else(|| "N/A", |s| s.as_str()),
                true,
            )
            .footer(footer);

        if let Some(publisher) = &version.published_by {
            let author = CreateEmbedAuthor::new(&publisher.login)
                .url(&publisher.url)
                .icon_url(&publisher.avatar);

            embed = embed.author(author).thumbnail(&publisher.avatar);
        }
    } else {
        let err = resp.json::<Error>().await;
        let err = err?;
        let err = err
            .errors
            .into_iter()
            .map(|d| ("❌ Error", format!("```{}```", d.detail), false));
        embed = embed.fields(err);
    }

    ctx.send(CreateReply::default().embed(embed)).await?;

    Ok(())
}
