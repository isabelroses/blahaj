use color_eyre::eyre::Result;
use nixpkgs_track::{branch_contains_commit, fetch_nixpkgs_pull_request};
use poise::{serenity_prelude::CreateEmbed, CreateReply};
use std::env;

use crate::Context;

const BRANCHES: [&str; 5] = [
    "master",
    "staging",
    "nixpkgs-unstable",
    "nixos-unstable-small",
    "nixos-unstable",
];

/// Track nixpkgs PRs
#[poise::command(slash_command)]
pub async fn nixpkgs(
    ctx: Context<'_>,
    #[description = "pr"]
    #[min = 0]
    pr: u64,
) -> Result<()> {
    ctx.defer().await?;

    let github_token = env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not set");

    let pull_request = tokio::task::spawn_blocking({
        let github_token = github_token.clone();
        move || fetch_nixpkgs_pull_request(pr, Some(&github_token))
    })
    .await??;

    let Some(commit_sha) = pull_request.merge_commit_sha else {
        ctx.say("This pull request is very old. I can't track it!")
            .await?;
        return Ok(());
    };

    let mut embed_description = String::new();
    for branch in BRANCHES {
        let github_token = github_token.clone();
        let commit_sha = commit_sha.clone();

        let has_pull_request = tokio::task::spawn_blocking(move || {
            branch_contains_commit(branch, &commit_sha, Some(&github_token))
        })
        .await
        .unwrap_or(Ok(false))?;

        embed_description.push_str(&format!(
            "{}: {}\n",
            branch,
            if has_pull_request { "✅" } else { "❌" }
        ));
    }

    let embed = CreateReply::default().embed(
        CreateEmbed::new()
            .title(format!("{} - #{}", pull_request.title, pull_request.number))
            .url(pull_request.html_url)
            .description(embed_description),
    );

    ctx.send(embed).await?;
    Ok(())
}
