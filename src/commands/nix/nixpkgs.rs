use color_eyre::eyre::Result;
use nixpkgs_track_lib::{branch_contains_commit, fetch_nixpkgs_pull_request};
use poise::{CreateReply, serenity_prelude::CreateEmbed};
use regex::Regex;
use reqwest::Client;
use std::fmt::Write as _;

use crate::types::Context;

pub static ROLLING_BRANCHES: [&str; 6] = [
    "staging",
    "staging-next",
    "master",
    "nixpkgs-unstable",
    "nixos-unstable-small",
    "nixos-unstable",
];
pub static STABLE_BRANCHES_TEMPLATE: [&str; 6] = [
    "staging-XX.XX",
    "staging-next-XX.XX",
    "release-XX.XX",
    "nixpkgs-XX.XX-darwin",
    "nixos-XX.XX-small",
    "nixos-XX.XX",
];

pub fn tracked_branches_for(base_ref: &str) -> Vec<String> {
    if ROLLING_BRANCHES.contains(&base_ref) {
        return ROLLING_BRANCHES.iter().map(|s| (*s).to_string()).collect();
    }

    let stable_version_regex = Regex::new(r"[0-9]+\.[0-9]+$").unwrap();
    if let Some(stable_version) = stable_version_regex.find(base_ref) {
        return STABLE_BRANCHES_TEMPLATE
            .iter()
            .map(|s| s.replace("XX.XX", stable_version.as_str()))
            .collect();
    }

    ROLLING_BRANCHES.iter().map(|s| (*s).to_string()).collect()
}

pub async fn branch_statuses(
    client: Client,
    github_token: &str,
    branches: &[String],
    commit_sha: &str,
) -> Result<Vec<(String, bool)>> {
    let mut statuses = Vec::with_capacity(branches.len());
    for branch in branches {
        let contains = branch_contains_commit(
            crate::types::W(client.clone()),
            branch,
            commit_sha,
            Some(github_token),
        )
        .await?;
        statuses.push((branch.clone(), contains));
    }
    Ok(statuses)
}

pub fn format_branch_statuses(statuses: &[(String, bool)]) -> String {
    let mut description = String::new();
    for (branch, contains) in statuses {
        let _ = writeln!(
            description,
            "{}: {}",
            branch,
            if *contains { "✅" } else { "❌" }
        );
    }
    description
}

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

    let Some(commit_sha) = pull_request.merge_commit_sha else {
        ctx.say("This pull request is very old. I can't track it!")
            .await?;
        return Ok(());
    };

    let branches = tracked_branches_for(&pull_request.base.r#ref);
    let statuses = branch_statuses(
        ctx.data().client.clone(),
        &ctx.data().github_token,
        &branches,
        &commit_sha,
    )
    .await?;

    let embed = CreateReply::default().embed(
        CreateEmbed::new()
            .title(format!("{} - #{}", pull_request.title, pull_request.number))
            .url(pull_request.html_url)
            .description(format_branch_statuses(&statuses)),
    );

    ctx.send(embed).await?;
    Ok(())
}
