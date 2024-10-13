use color_eyre::eyre::Result;
use git_tracker::TrackedRepository;
use poise::{serenity_prelude::CreateEmbed, CreateReply};
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
    let tracker = TrackedRepository::new(
        nixpkgs_path.into(),
        NIXPKGS_URL.to_string(),
        "origin".to_string(),
    );

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

    let branchs = vec![
        "master".to_string(),
        "staging".to_string(),
        "nixpkgs-unstable".to_string(),
        "nixos-unstable-small".to_string(),
        "nixos-unstable".to_string(),
    ];

    let prd_branches = tracker.branches_contain_sha(&branchs, &commit_sha)?;

    // if we don't find the commit in any branches from above, we can pretty safely assume
    // it's an unmerged PR
    let embed_description: String = if prd_branches.is_empty() {
        "It doesn't look like this PR has been merged yet! (or maybe I just haven't updated)"
            .to_string()
    } else {
        prd_branches
            .iter()
            .fold(String::new(), |acc, (name, has_commit)| {
                let emoji = if *has_commit { "✅" } else { "❌" };
                format!("{acc}\n{name} {emoji}")
            })
    };

    let embed = CreateReply::default().embed(
        CreateEmbed::new()
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
