//! Tracked PRs: users waiting for a nixpkgs PR to reach a branch, and the
//! poller that tells them when it has.
//!
//! The poller is careful with GitHub requests. Each cycle it checks each PR
//! once, however many people track it. It looks up an unmerged PR's merge
//! state and nothing more, remembers the merge commit once there is one, and
//! then checks only the branches people are waiting for: usually one request
//! per PR per cycle.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use nixpkgs_track_lib::NixpkgsTrackError;
use poise::serenity_prelude::{
    ChannelId, Context as SerenityContext, CreateEmbed, CreateMessage, UserId,
};
use reqwest::Client;
use rusqlite::{Connection, params};

use super::pr::{self, BranchStatus, Merge, PullRequest};
use crate::db;

const POLL_INTERVAL_SECS: u64 = 600;
const PER_PR_DELAY_SECS: u64 = 2;
const TRACKING_TTL_SECS: i64 = 60 * 60 * 24 * 30;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackedPr {
    pub pr_number: u64,
    pub user_id: u64,
    pub channel_id: u64,
    pub created_at: i64,
    pub target_branch: String,
    /// Remembered once the PR is merged, so the poller stops fetching the PR.
    pub merge_commit_sha: Option<String>,
}

pub fn init(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS tracked_prs (
            pr_number INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            channel_id INTEGER NOT NULL,
            created_at INTEGER NOT NULL,
            target_branch TEXT NOT NULL DEFAULT 'nixpkgs-unstable',
            merge_commit_sha TEXT,
            PRIMARY KEY (pr_number, user_id)
        )",
        [],
    )?;

    if !db::column_exists(conn, "tracked_prs", "target_branch")? {
        conn.execute(
            "ALTER TABLE tracked_prs ADD COLUMN target_branch TEXT NOT NULL DEFAULT 'nixpkgs-unstable'",
            [],
        )?;
    }
    if !db::column_exists(conn, "tracked_prs", "merge_commit_sha")? {
        conn.execute(
            "ALTER TABLE tracked_prs ADD COLUMN merge_commit_sha TEXT",
            [],
        )?;
    }

    Ok(())
}

/// Starts (or restarts) tracking a PR for a user.
pub fn track(conn: &Connection, tracked: &TrackedPr) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO tracked_prs (pr_number, user_id, channel_id, created_at, target_branch, merge_commit_sha) VALUES (?, ?, ?, ?, ?, ?)",
        params![
            tracked.pr_number.cast_signed(),
            tracked.user_id.cast_signed(),
            tracked.channel_id.cast_signed(),
            tracked.created_at,
            tracked.target_branch,
            tracked.merge_commit_sha,
        ],
    )?;
    Ok(())
}

fn all(conn: &Connection) -> rusqlite::Result<Vec<TrackedPr>> {
    conn.prepare(
        "SELECT pr_number, user_id, channel_id, created_at, target_branch, merge_commit_sha FROM tracked_prs",
    )?
    .query_map([], |row| {
        Ok(TrackedPr {
            pr_number: row.get::<_, i64>(0)?.cast_unsigned(),
            user_id: row.get::<_, i64>(1)?.cast_unsigned(),
            channel_id: row.get::<_, i64>(2)?.cast_unsigned(),
            created_at: row.get(3)?,
            target_branch: row.get(4)?,
            merge_commit_sha: row.get(5)?,
        })
    })?
    .collect()
}

/// Remembers PR `pr_number`'s merge commit for everyone tracking it.
fn set_merge_commit(conn: &Connection, pr_number: u64, sha: &str) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE tracked_prs SET merge_commit_sha = ? WHERE pr_number = ?",
        params![sha, pr_number.cast_signed()],
    )?;
    Ok(())
}

fn forget(conn: &Connection, pr_number: u64, user_id: u64) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM tracked_prs WHERE pr_number = ? AND user_id = ?",
        params![pr_number.cast_signed(), user_id.cast_signed()],
    )?;
    Ok(())
}

fn is_expired(tracked: &TrackedPr, now: i64) -> bool {
    now - tracked.created_at > TRACKING_TTL_SECS
}

/// Groups unexpired rows by PR, so each PR is checked once however many people
/// track it. Returns the expired rows separately.
fn group_by_pr(rows: Vec<TrackedPr>, now: i64) -> (BTreeMap<u64, Vec<TrackedPr>>, Vec<TrackedPr>) {
    let mut groups: BTreeMap<u64, Vec<TrackedPr>> = BTreeMap::new();
    let mut expired = Vec::new();
    for row in rows {
        if is_expired(&row, now) {
            expired.push(row);
        } else {
            groups.entry(row.pr_number).or_default().push(row);
        }
    }
    (groups, expired)
}

async fn poll_once(serenity: &SerenityContext, client: &Client, github_token: &str) {
    let rows = match db::with(all) {
        Ok(rows) => rows,
        Err(err) => {
            eprintln!("nixpkgs-track: failed to load tracked PRs: {err}");
            return;
        }
    };

    let (groups, expired) = group_by_pr(rows, chrono::Utc::now().timestamp());
    for row in &expired {
        forget_logged(row);
    }

    for (pr_number, rows) in groups {
        match poll_pr(serenity, client, github_token, pr_number, &rows).await {
            Ok(()) => {}
            Err(NixpkgsTrackError::PullRequestNotFound(_)) => rows.iter().for_each(forget_logged),
            Err(NixpkgsTrackError::RateLimitExceeded) => {
                eprintln!("nixpkgs-track: rate limited; aborting this poll cycle");
                return;
            }
            Err(err) => eprintln!("nixpkgs-track: error checking PR #{pr_number}: {err}"),
        }

        tokio::time::sleep(std::time::Duration::from_secs(PER_PR_DELAY_SECS)).await;
    }
}

/// Checks one PR for everyone tracking it, and notifies those whose target
/// branch it has reached.
async fn poll_pr(
    serenity: &SerenityContext,
    client: &Client,
    github_token: &str,
    pr_number: u64,
    rows: &[TrackedPr],
) -> Result<(), NixpkgsTrackError> {
    let sha = if let Some(sha) = rows.iter().find_map(|r| r.merge_commit_sha.clone()) {
        sha
    } else {
        let Merge::Merged(sha) = pr::fetch(client, github_token, pr_number).await?.merge else {
            return Ok(());
        };
        if let Err(err) = db::with(|conn| set_merge_commit(conn, pr_number, &sha)) {
            eprintln!("nixpkgs-track: failed to remember PR #{pr_number}'s merge commit: {err}");
        }
        sha
    };

    let targets: BTreeSet<&str> = rows.iter().map(|r| r.target_branch.as_str()).collect();
    let mut reached = BTreeSet::new();
    for target in targets {
        if pr::contains(client, github_token, target, &sha).await? {
            reached.insert(target);
        }
    }
    if reached.is_empty() {
        return Ok(());
    }

    // Someone's target was reached. This happens once per PR, so it's worth
    // the extra requests to show the full picture in the notification.
    let pr = pr::fetch(client, github_token, pr_number).await?;
    let statuses = pr::statuses(client, github_token, pr.tracked_branches(), &sha).await?;
    for row in rows
        .iter()
        .filter(|r| reached.contains(r.target_branch.as_str()))
    {
        notify(serenity, row, &pr, &statuses).await;
        forget_logged(row);
    }

    Ok(())
}

fn forget_logged(tracked: &TrackedPr) {
    if let Err(err) = db::with(|conn| forget(conn, tracked.pr_number, tracked.user_id)) {
        eprintln!(
            "nixpkgs-track: failed to stop tracking PR #{}: {err}",
            tracked.pr_number
        );
    }
}

/// DMs the user, falling back to pinging them in the channel they tracked from.
async fn notify(
    serenity: &SerenityContext,
    tracked: &TrackedPr,
    pr: &PullRequest,
    statuses: &[BranchStatus],
) {
    let mut description = pr::describe(statuses);
    let _ = write!(description, "\nReached `{}`.", tracked.target_branch);

    let embed = CreateEmbed::new()
        .title(pr.heading())
        .url(&pr.url)
        .description(description)
        .color(0x4D_6F_B7);

    let user = UserId::new(tracked.user_id);
    let dm_sent = match user.create_dm_channel(serenity).await {
        Ok(dm) => dm
            .send_message(serenity, CreateMessage::new().embed(embed.clone()))
            .await
            .is_ok(),
        Err(_) => false,
    };

    if !dm_sent {
        let _ = ChannelId::new(tracked.channel_id)
            .send_message(
                serenity,
                CreateMessage::new()
                    .content(format!("<@{}>", tracked.user_id))
                    .embed(embed),
            )
            .await;
    }
}

/// Checks every tracked PR every [`POLL_INTERVAL_SECS`].
pub fn spawn_poller(serenity: SerenityContext, client: Client, github_token: String) {
    tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(POLL_INTERVAL_SECS));
        loop {
            interval.tick().await;
            poll_once(&serenity, &client, &github_token).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tracked(pr_number: u64, user_id: u64, created_at: i64) -> TrackedPr {
        TrackedPr {
            pr_number,
            user_id,
            channel_id: 8,
            created_at,
            target_branch: "nixpkgs-unstable".to_string(),
            merge_commit_sha: None,
        }
    }

    #[test]
    fn each_pr_is_checked_once_and_expired_rows_split_off() {
        let now = 100 + TRACKING_TTL_SECS;
        let rows = vec![
            tracked(1, 7, 100),
            tracked(1, 8, 100),
            tracked(2, 7, 100),
            tracked(3, 7, 99), // just past the TTL
        ];

        let (groups, expired) = group_by_pr(rows, now);
        assert_eq!(groups.keys().copied().collect::<Vec<_>>(), vec![1, 2]);
        assert_eq!(groups[&1].len(), 2);
        assert_eq!(expired, vec![tracked(3, 7, 99)]);
    }

    #[test]
    fn merge_commit_is_remembered_for_everyone_tracking_the_pr() {
        let conn = crate::db::test_connection();
        track(&conn, &tracked(1, 7, 100)).unwrap();
        track(&conn, &tracked(1, 8, 100)).unwrap();
        track(&conn, &tracked(2, 7, 100)).unwrap();

        set_merge_commit(&conn, 1, "abc").unwrap();

        let shas: Vec<(u64, Option<String>)> = all(&conn)
            .unwrap()
            .into_iter()
            .map(|r| (r.pr_number, r.merge_commit_sha))
            .collect();
        assert!(shas.contains(&(1, Some("abc".to_string()))));
        assert!(!shas.contains(&(1, None)));
        assert!(shas.contains(&(2, None)));
    }

    #[test]
    fn store_round_trip() {
        let conn = crate::db::test_connection();
        let row = TrackedPr {
            merge_commit_sha: Some("abc".to_string()),
            ..tracked(42, 7, 1_000)
        };
        track(&conn, &row).unwrap();
        track(&conn, &row).unwrap(); // re-tracking replaces
        assert_eq!(all(&conn).unwrap(), vec![row.clone()]);
        forget(&conn, row.pr_number, row.user_id).unwrap();
        assert_eq!(all(&conn).unwrap(), Vec::<TrackedPr>::new());
    }

    #[test]
    fn upgrades_a_table_without_the_merge_commit_column() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE tracked_prs (
                pr_number INTEGER NOT NULL,
                user_id INTEGER NOT NULL,
                channel_id INTEGER NOT NULL,
                created_at INTEGER NOT NULL,
                target_branch TEXT NOT NULL DEFAULT 'nixpkgs-unstable',
                PRIMARY KEY (pr_number, user_id)
            )",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO tracked_prs VALUES (42, 7, 8, 1000, 'master')",
            [],
        )
        .unwrap();

        init(&conn).unwrap();
        assert_eq!(all(&conn).unwrap()[0].merge_commit_sha, None);
    }
}
