use color_eyre::eyre::Result;
use nixpkgs_track_lib::fetch_nixpkgs_pull_request;
use poise::serenity_prelude::{
    Context as SerenityContext, CreateEmbed, CreateMessage, UserId,
};
use poise::CreateReply;
use reqwest::Client;

use crate::commands::nix::nixpkgs::{
    branch_statuses, format_branch_statuses, resolve_target_branch, tracked_branches_for,
    TargetBranch,
};
use crate::types::Context;
use crate::utils::TRACKED_PRS_DB;

const POLL_INTERVAL_SECS: u64 = 600;
const PER_PR_DELAY_SECS: u64 = 2;
const TRACKING_TTL_SECS: i64 = 60 * 60 * 24 * 30;

/// Track a nixpkgs PR and get a DM when it lands in the chosen branch
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
) -> Result<()> {
    ctx.defer().await?;

    let pull_request = fetch_nixpkgs_pull_request(
        crate::types::W(ctx.data().client.clone()),
        pr,
        Some(&ctx.data().github_token),
    )
    .await?;

    let title = format!("{} - #{}", pull_request.title, pull_request.number);
    let url = pull_request.html_url.clone();

    let Some(commit_sha) = pull_request.merge_commit_sha.clone() else {
        let embed = CreateReply::default().embed(
            CreateEmbed::new()
                .title(title)
                .url(url)
                .description("This pull request is very old. I can't track it!"),
        );
        ctx.send(embed).await?;
        return Ok(());
    };

    let target_branch = resolve_target_branch(&pull_request.base.r#ref, branch);
    let branches = tracked_branches_for(&pull_request.base.r#ref);
    let statuses = branch_statuses(
        ctx.data().client.clone(),
        &ctx.data().github_token,
        &branches,
        &commit_sha,
    )
    .await?;

    let reached_target = statuses
        .iter()
        .find(|(name, _)| *name == target_branch)
        .is_some_and(|(_, contains)| *contains);
    let mut description = format_branch_statuses(&statuses);

    if reached_target {
        description.push_str(&format!(
            "\nAlready propagated to `{target_branch}` — nothing to track."
        ));
    } else {
        let user_id = ctx.author().id.get();
        let channel_id = ctx.channel_id().get();
        let now = chrono::Utc::now().timestamp();

        tokio::task::block_in_place(|| {
            let conn = TRACKED_PRS_DB.lock().unwrap();
            conn.execute(
                "INSERT OR REPLACE INTO tracked_prs (pr_number, user_id, channel_id, created_at, target_branch) VALUES (?, ?, ?, ?, ?)",
                rusqlite::params![
                    pr.cast_signed(),
                    user_id.cast_signed(),
                    channel_id.cast_signed(),
                    now,
                    target_branch,
                ],
            )
        })?;

        description.push_str(&format!(
            "\nI'll DM you when this PR reaches `{target_branch}`."
        ));
    }

    let embed = CreateReply::default().embed(
        CreateEmbed::new()
            .title(title)
            .url(url)
            .description(description),
    );
    ctx.send(embed).await?;
    Ok(())
}

#[derive(Debug, Clone)]
struct TrackedRow {
    pr_number: u64,
    user_id: u64,
    channel_id: u64,
    created_at: i64,
    target_branch: String,
}

fn load_tracked_rows() -> Vec<TrackedRow> {
    tokio::task::block_in_place(|| {
        let conn = TRACKED_PRS_DB.lock().unwrap();
        let Ok(mut stmt) = conn.prepare(
            "SELECT pr_number, user_id, channel_id, created_at, target_branch FROM tracked_prs",
        ) else {
            return Vec::new();
        };
        stmt.query_map([], |row| {
            Ok(TrackedRow {
                pr_number: row.get::<_, i64>(0)?.cast_unsigned(),
                user_id: row.get::<_, i64>(1)?.cast_unsigned(),
                channel_id: row.get::<_, i64>(2)?.cast_unsigned(),
                created_at: row.get::<_, i64>(3)?,
                target_branch: row.get::<_, String>(4)?,
            })
        })
        .map(|iter| iter.filter_map(std::result::Result::ok).collect::<Vec<_>>())
        .unwrap_or_default()
    })
}

fn delete_tracked(pr_number: u64, user_id: u64) {
    tokio::task::block_in_place(|| {
        let conn = TRACKED_PRS_DB.lock().unwrap();
        let _ = conn.execute(
            "DELETE FROM tracked_prs WHERE pr_number = ? AND user_id = ?",
            rusqlite::params![pr_number.cast_signed(), user_id.cast_signed()],
        );
    });
}

async fn notify_reached_target(
    serenity: &SerenityContext,
    row: &TrackedRow,
    pr: &nixpkgs_track_lib::PullRequest,
    statuses: &[(String, bool)],
) {
    let mut description = format_branch_statuses(statuses);
    description.push_str(&format!("\nReached `{}`.", row.target_branch));

    let embed = CreateEmbed::new()
        .title(format!("{} - #{}", pr.title, pr.number))
        .url(&pr.html_url)
        .description(description);

    let user = UserId::new(row.user_id);
    let dm_sent = match user.create_dm_channel(serenity).await {
        Ok(dm) => dm
            .send_message(serenity, CreateMessage::new().embed(embed.clone()))
            .await
            .is_ok(),
        Err(_) => false,
    };

    if !dm_sent {
        let channel = poise::serenity_prelude::ChannelId::new(row.channel_id);
        let _ = channel
            .send_message(
                serenity,
                CreateMessage::new()
                    .content(format!("<@{}>", row.user_id))
                    .embed(embed),
            )
            .await;
    }
}

pub async fn poll_once(serenity: &SerenityContext) {
    let rows = load_tracked_rows();
    if rows.is_empty() {
        return;
    }

    let github_token = crate::config::get().github_token.clone();
    let Ok(client) = Client::builder().user_agent("isabelroses/blahaj").build() else {
        return;
    };

    let now = chrono::Utc::now().timestamp();

    for row in rows {
        if now - row.created_at > TRACKING_TTL_SECS {
            delete_tracked(row.pr_number, row.user_id);
            continue;
        }

        let pr = fetch_nixpkgs_pull_request(
            crate::types::W(client.clone()),
            row.pr_number,
            Some(&github_token),
        )
        .await;

        match pr {
            Ok(pr) => {
                let Some(commit_sha) = pr.merge_commit_sha.clone() else {
                    tokio::time::sleep(std::time::Duration::from_secs(PER_PR_DELAY_SECS)).await;
                    continue;
                };

                let branches = tracked_branches_for(&pr.base.r#ref);
                let statuses =
                    match branch_statuses(client.clone(), &github_token, &branches, &commit_sha)
                        .await
                    {
                        Ok(s) => s,
                        Err(err) => {
                            eprintln!(
                                "nixpkgs-track: error checking branches for PR #{}: {err}",
                                row.pr_number
                            );
                            tokio::time::sleep(std::time::Duration::from_secs(PER_PR_DELAY_SECS))
                                .await;
                            continue;
                        }
                    };

                let reached_target = statuses
                    .iter()
                    .find(|(name, _)| *name == row.target_branch)
                    .is_some_and(|(_, contains)| *contains);

                if reached_target {
                    notify_reached_target(serenity, &row, &pr, &statuses).await;
                    delete_tracked(row.pr_number, row.user_id);
                }
            }
            Err(nixpkgs_track_lib::NixpkgsTrackError::PullRequestNotFound(_)) => {
                delete_tracked(row.pr_number, row.user_id);
            }
            Err(nixpkgs_track_lib::NixpkgsTrackError::RateLimitExceeded) => {
                eprintln!("nixpkgs-track: rate limited; aborting this poll cycle");
                return;
            }
            Err(err) => {
                eprintln!("nixpkgs-track: error fetching PR #{}: {err}", row.pr_number);
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(PER_PR_DELAY_SECS)).await;
    }
}

pub fn spawn_poller(serenity: SerenityContext) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(POLL_INTERVAL_SECS));
        loop {
            interval.tick().await;
            poll_once(&serenity).await;
        }
    });
}
