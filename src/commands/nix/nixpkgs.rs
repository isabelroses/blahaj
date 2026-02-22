use color_eyre::eyre::Result;
use nixpkgs_track_lib::{branch_contains_commit, fetch_nixpkgs_pull_request};
use poise::{CreateReply, serenity_prelude::CreateEmbed};
use regex::Regex;
use std::fmt::Write as _;

use crate::types::Context;

static ROLLING_BRANCHES: [&str; 6] = [
    "staging",
    "staging-next",
    "master",
    "nixpkgs-unstable",
    "nixos-unstable-small",
    "nixos-unstable",
];
static STABLE_BRANCHES_TEMPLATE: [&str; 6] = [
    "staging-XX.XX",
    "staging-next-XX.XX",
    "release-XX.XX",
    "nixpkgs-XX.XX-darwin",
    "nixos-XX.XX-small",
    "nixos-XX.XX",
];

/// Track nixpkgs PRs
#[poise::command(
    slash_command,
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
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

    let merged_into_branch = pull_request.base.r#ref;

    let Some(commit_sha) = pull_request.merge_commit_sha else {
        ctx.say("This pull request is very old. I can't track it!")
            .await?;
        return Ok(());
    };

    let stable_branches: Option<Vec<String>> =
        if ROLLING_BRANCHES.contains(&merged_into_branch.as_str()) {
            None
        } else {
            // regex for stable version XX.XX
            let stable_version_regex = Regex::new(r"[0-9]+\.[0-9]+$").unwrap();
            if let Some(stable_version) = stable_version_regex.find(&merged_into_branch) {
                let stable_branches = STABLE_BRANCHES_TEMPLATE
                    .iter()
                    .map(|s| s.replace("XX.XX", stable_version.as_str()))
                    .collect();
                Some(stable_branches)
            } else {
                None
            }
        };

    let tracked_branches = match stable_branches {
        Some(ref stable_branches) => stable_branches
            .iter()
            .map(std::string::String::as_str)
            .collect(),
        None => Vec::from(ROLLING_BRANCHES),
    };

    let mut embed_description = String::new();
    for branch in tracked_branches {
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
