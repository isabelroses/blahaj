use std::fmt::Write;

use color_eyre::eyre::Result;
use poise::CreateReply;
use poise::serenity_prelude::CreateEmbed;

use crate::db;
use crate::nixpkgs::pr::{self, Merge, TargetBranch};
use crate::nixpkgs::tracking::{self, TrackedPr};
use crate::types::Context;

/// Show which branches a nixpkgs PR has reached, and get a DM when it lands
#[poise::command(
    slash_command,
    rename = "nixpkgs-track",
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn nixpkgs_track(
    ctx: Context<'_>,
    #[description = "pr"]
    #[min = 0]
    pr: u64,
    #[description = "branch to wait for (defaults to nixpkgs-unstable or the release branch)"]
    branch: Option<TargetBranch>,
    #[description = "DM you when it reaches the branch (default: true)"] notify: Option<bool>,
) -> Result<()> {
    ctx.defer().await?;

    let client = &ctx.data().client;
    let token = &ctx.data().github_token;
    let pull_request = pr::fetch(client, token, pr).await?;
    let embed = CreateEmbed::new()
        .title(pull_request.heading())
        .url(&pull_request.url);
    let target_branch = pull_request.target_branch(branch);

    // Only a merged PR has a commit worth looking for on the branches, so an
    // unmerged one costs a single request.
    let (mut description, reached, merge_commit_sha) = match &pull_request.merge {
        Merge::NoMergeCommit => {
            let embed = embed
                .description("This pull request is very old. I can't track it!")
                .color(0xF3_8B_A8);
            ctx.send(CreateReply::default().embed(embed)).await?;
            return Ok(());
        }
        Merge::NotMerged => ("Not merged yet.\n".to_string(), false, None),
        Merge::Merged(sha) => {
            let statuses =
                pr::statuses(client, token, pull_request.tracked_branches(), sha).await?;
            (
                pr::describe(&statuses),
                pr::reached(&statuses, &target_branch),
                Some(sha.clone()),
            )
        }
    };

    if notify.unwrap_or(true) {
        if reached {
            let _ = write!(
                description,
                "\nAlready propagated to `{target_branch}` — nothing to track."
            );
        } else {
            let tracked = TrackedPr {
                pr_number: pr,
                user_id: ctx.author().id.get(),
                channel_id: ctx.channel_id().get(),
                created_at: chrono::Utc::now().timestamp(),
                target_branch: target_branch.clone(),
                merge_commit_sha,
            };
            db::with(|conn| tracking::track(conn, &tracked))?;

            let _ = write!(
                description,
                "\nI'll DM you when this PR reaches `{target_branch}`."
            );
        }
    }

    ctx.send(CreateReply::default().embed(embed.description(description).color(0x4D_6F_B7)))
        .await?;
    Ok(())
}
