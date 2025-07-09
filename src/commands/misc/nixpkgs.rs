use color_eyre::eyre::Result;
use nixpkgs_track_lib::{branch_contains_commit, fetch_nixpkgs_pull_request};
use poise::{serenity_prelude::CreateEmbed, CreateReply};
use std::fmt::Write as _;

use crate::types::Context;

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

    let pull_request = fetch_nixpkgs_pull_request(
        crate::types::W(ctx.data().client.clone()),
        pr,
        Some(&ctx.data().github_token),
    )
    .await?;

    let Some(commit_sha) = pull_request.merge_commit_sha else {
        ctx.say("This pull request is very old. I can't track it!")
            .await?;
        return Ok(());
    };

    let mut embed_description = String::new();
    for branch in BRANCHES {
        let github_token = ctx.data().github_token.clone();
        let commit_sha = commit_sha.clone();

        let has_pull_request = branch_contains_commit(
            crate::types::W(ctx.data().client.clone()),
            branch,
            &commit_sha,
            Some(&github_token),
        )
        .await?;

        let _ = writeln!(
            embed_description,
            "{}: {}",
            branch,
            if has_pull_request { "✅" } else { "❌" }
        );
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
