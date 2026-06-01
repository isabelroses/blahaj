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

/// The stable release version (e.g. `24.11`) targeted by `base_ref`, if any.
fn stable_version(base_ref: &str) -> Option<String> {
    if ROLLING_BRANCHES.contains(&base_ref) {
        return None;
    }

    let stable_version_regex = Regex::new(r"[0-9]+\.[0-9]+$").unwrap();
    stable_version_regex
        .find(base_ref)
        .map(|m| m.as_str().to_string())
}

pub fn tracked_branches_for(base_ref: &str) -> Vec<String> {
    if let Some(version) = stable_version(base_ref) {
        return STABLE_BRANCHES_TEMPLATE
            .iter()
            .map(|s| s.replace("XX.XX", &version))
            .collect();
    }

    ROLLING_BRANCHES.iter().map(|s| (*s).to_string()).collect()
}

/// A branch a user can ask to be notified about. The variants line up with the
/// shared ordering of [`ROLLING_BRANCHES`] and [`STABLE_BRANCHES_TEMPLATE`], so
/// the same choice resolves to the right concrete branch for either a rolling
/// or a stable PR.
#[derive(Debug, Clone, Copy, poise::ChoiceParameter)]
pub enum TargetBranch {
    #[name = "staging"]
    Staging,
    #[name = "staging-next"]
    StagingNext,
    #[name = "master / release"]
    MasterOrRelease,
    #[name = "nixpkgs-unstable / darwin"]
    UnstableOrDarwin,
    #[name = "small"]
    Small,
    #[name = "nixos-unstable / nixos release"]
    Unstable,
}

impl TargetBranch {
    fn index(self) -> usize {
        match self {
            TargetBranch::Staging => 0,
            TargetBranch::StagingNext => 1,
            TargetBranch::MasterOrRelease => 2,
            TargetBranch::UnstableOrDarwin => 3,
            TargetBranch::Small => 4,
            TargetBranch::Unstable => 5,
        }
    }
}

/// Resolve the concrete branch a PR should be tracked until. When the user
/// doesn't pick one, this defaults to `nixpkgs-unstable` for rolling PRs or the
/// `release-XX.XX` branch for stable ones.
pub fn resolve_target_branch(base_ref: &str, choice: Option<TargetBranch>) -> String {
    let branches = tracked_branches_for(base_ref);
    let index = match choice {
        Some(target) => target.index(),
        None if stable_version(base_ref).is_some() => 2, // release-XX.XX
        None => 3,                                        // nixpkgs-unstable
    };

    branches
        .get(index)
        .or_else(|| branches.last())
        .cloned()
        .unwrap_or_default()
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
