use color_eyre::eyre::Result;
use git_tracker::Tracker;
use poise::{serenity_prelude as serenity, CreateReply};
use serde::Deserialize;
use std::env;

use crate::Context;

const NIXPKGS_URL: &str = "https://github.com/NixOS/nixpkgs";

/// Track nixpkgs PRs
#[poise::command(slash_command)]
pub async fn nixpkgs(
    ctx: Context<'_>,
    #[description = "pr"]
    #[min = 0]
    pr: i32,
) -> Result<()> {
    ctx.defer().await?;

    let nixpkgs_path = env::var("NIXPKGS").expect("NIXPKGS not set");
    let github_token = env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not set");
    let tracker = Tracker::from_path(&nixpkgs_path)?;

	let client = &ctx.data().client;

    // find out what commit our PR was merged in
    let Some(commit_sha) = ({
        let url = format!("https://api.github.com/repos/nixos/nixpkgs/pulls/{pr}");
        let resp = client
            .get(&url)
            .header("User-Agent", "blahaj")
            .header("Authorization", format!("Bearer {github_token}"))
            .send()
            .await
            .expect("error fetching")
            .json::<PullRequest>()
            .await?;

        resp.merge_commit_sha
    }) else {
        ctx.say("It seems this pull request is very old. I can't track it")
            .await?;
        return Ok(());
    };

    let mut status_results = vec![];
    for branch_name in &[
        "master",
        "staging",
        "nixpkgs-unstable",
        "nixos-unstable-small",
        "nixos-unstable",
    ] {
        let full_branch_name = format!("origin/{branch_name}");
        let has_pr = tracker.branch_contains_sha(&full_branch_name, &commit_sha)?;

        if has_pr {
            status_results.push(format!("`{branch_name}` ✅"));
        } else {
            status_results.push(format!("`{branch_name}` ❌"));
        }
    }

    // if we don't find the commit in any branches from above, we can pretty safely assume
    // it's an unmerged PR
    let embed_description: String = if status_results.is_empty() {
        "It doesn't look like this PR has been merged yet! (or maybe I just haven't updated)"
            .to_string()
    } else {
        status_results.join("\n")
    };

    let embed = CreateReply::default().embed(
        serenity::CreateEmbed::new()
            .title(format!("Nixpkgs PR #{pr} Status"))
            .url(format!("{NIXPKGS_URL}/pull/{pr}"))
            .description(embed_description),
    );

    ctx.send(embed).await?;
    Ok(())
}

#[derive(Clone, Debug, Deserialize)]
struct PullRequest {
    merge_commit_sha: Option<String>,
}
