use color_eyre::eyre::Result;
use nixpkgs_track_lib::fetch_nixpkgs_pull_request;
use poise::serenity_prelude::{
    Context as SerenityContext, CreateEmbed, CreateMessage, UserId,
};
use poise::CreateReply;
use reqwest::Client;

use crate::types::Context;
use crate::utils::TRACKED_PRS_DB;

const POLL_INTERVAL_SECS: u64 = 600;
const PER_PR_DELAY_SECS: u64 = 2;
const TRACKING_TTL_SECS: i64 = 60 * 60 * 24 * 30;

/// Track a nixpkgs PR and get a DM when it's merged
#[poise::command(
    slash_command,
    rename = "track-nixpkgs",
    install_context = "Guild|User",
    interaction_context = "Guild|BotDm|PrivateChannel"
)]
pub async fn track_nixpkgs(
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

    if pull_request.merged_at.is_some() {
        let embed = CreateReply::default().embed(
            CreateEmbed::new()
                .title(format!("{} - #{}", pull_request.title, pull_request.number))
                .url(&pull_request.html_url)
                .description("This PR is already merged. Use `/nixpkgs` to see branch propagation."),
        );
        ctx.send(embed).await?;
        return Ok(());
    }

    let user_id = ctx.author().id.get();
    let channel_id = ctx.channel_id().get();
    let now = chrono::Utc::now().timestamp();

    tokio::task::block_in_place(|| {
        let conn = TRACKED_PRS_DB.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO tracked_prs (pr_number, user_id, channel_id, created_at) VALUES (?, ?, ?, ?)",
            rusqlite::params![
                pr.cast_signed(),
                user_id.cast_signed(),
                channel_id.cast_signed(),
                now,
            ],
        )
    })?;

    let embed = CreateReply::default().embed(
        CreateEmbed::new()
            .title(format!("Tracking: {} - #{}", pull_request.title, pull_request.number))
            .url(&pull_request.html_url)
            .description("I'll DM you when this PR is merged."),
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
}

fn load_tracked_rows() -> Vec<TrackedRow> {
    tokio::task::block_in_place(|| {
        let conn = TRACKED_PRS_DB.lock().unwrap();
        let Ok(mut stmt) =
            conn.prepare("SELECT pr_number, user_id, channel_id, created_at FROM tracked_prs")
        else {
            return Vec::new();
        };
        stmt.query_map([], |row| {
            Ok(TrackedRow {
                pr_number: row.get::<_, i64>(0)?.cast_unsigned(),
                user_id: row.get::<_, i64>(1)?.cast_unsigned(),
                channel_id: row.get::<_, i64>(2)?.cast_unsigned(),
                created_at: row.get::<_, i64>(3)?,
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

async fn notify_merged(
    serenity: &SerenityContext,
    row: &TrackedRow,
    pr: &nixpkgs_track_lib::PullRequest,
) {
    let embed = CreateEmbed::new()
        .title(format!("Merged: {} - #{}", pr.title, pr.number))
        .url(&pr.html_url)
        .description("Your tracked nixpkgs PR has been merged.");

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
            Ok(pr) if pr.merged_at.is_some() => {
                notify_merged(serenity, &row, &pr).await;
                delete_tracked(row.pr_number, row.user_id);
            }
            Ok(_) => {}
            Err(nixpkgs_track_lib::NixpkgsTrackError::PullRequestNotFound(_)) => {
                delete_tracked(row.pr_number, row.user_id);
            }
            Err(nixpkgs_track_lib::NixpkgsTrackError::RateLimitExceeded) => {
                eprintln!("track-nixpkgs: rate limited; aborting this poll cycle");
                return;
            }
            Err(err) => {
                eprintln!("track-nixpkgs: error fetching PR #{}: {err}", row.pr_number);
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
