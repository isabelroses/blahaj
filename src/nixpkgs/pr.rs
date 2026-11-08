//! Which nixpkgs branches a pull request has reached.
//!
//! Each GitHub request is its own function, so callers can make only the
//! ones they need: [`fetch`] is one request, [`contains`] is one request, and
//! [`statuses`] is one request per branch.

use std::fmt::Write as _;
use std::sync::LazyLock;

use futures_util::future::join_all;
use nixpkgs_track_lib::{NixpkgsTrackError, branch_contains_commit, fetch_nixpkgs_pull_request};
use regex::Regex;
use reqwest::Client;

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

static STABLE_VERSION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[0-9]+\.[0-9]+$").unwrap());

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

/// Where a PR is in merging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Merge {
    /// Not merged (yet). Open PRs have a test merge commit that will never land,
    /// so there's nothing to look for on any branch.
    NotMerged,
    /// Merged as this commit, which is what propagates through the branches.
    Merged(String),
    /// Merged so long ago that GitHub doesn't report a merge commit.
    NoMergeCommit,
}

impl Merge {
    fn from_github(merged: bool, merge_commit_sha: Option<String>) -> Self {
        match (merged, merge_commit_sha) {
            (false, _) => Merge::NotMerged,
            (true, Some(sha)) => Merge::Merged(sha),
            (true, None) => Merge::NoMergeCommit,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub base_ref: String,
    pub merge: Merge,
}

impl PullRequest {
    pub fn heading(&self) -> String {
        format!("{} - #{}", self.title, self.number)
    }

    /// The branches this PR propagates through, in order.
    pub fn tracked_branches(&self) -> Vec<String> {
        tracked_branches_for(&self.base_ref)
    }

    /// The branch to wait for. Defaults to `nixpkgs-unstable` for rolling PRs
    /// or the `release-XX.XX` branch for stable ones.
    pub fn target_branch(&self, choice: Option<TargetBranch>) -> String {
        let branches = self.tracked_branches();
        let index = match choice {
            Some(target) => target.index(),
            None if stable_version(&self.base_ref).is_some() => 2, // release-XX.XX
            None => 3,                                             // nixpkgs-unstable
        };

        branches
            .get(index)
            .or_else(|| branches.last())
            .cloned()
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchStatus {
    pub name: String,
    pub contains: bool,
}

/// Fetches PR `number`. One request.
pub async fn fetch(
    client: &Client,
    github_token: &str,
    number: u64,
) -> Result<PullRequest, NixpkgsTrackError> {
    let pr = fetch_nixpkgs_pull_request(W(client.clone()), number, Some(github_token)).await?;
    Ok(PullRequest {
        number: pr.number,
        title: pr.title,
        url: pr.html_url,
        base_ref: pr.base.r#ref,
        merge: Merge::from_github(pr.merged, pr.merge_commit_sha),
    })
}

/// Whether `branch` contains `commit`. One request.
pub async fn contains(
    client: &Client,
    github_token: &str,
    branch: &str,
    commit: &str,
) -> Result<bool, NixpkgsTrackError> {
    branch_contains_commit(W(client.clone()), branch, commit, Some(github_token)).await
}

/// Whether each of `branches` contains `commit`, checked concurrently. One
/// request per branch.
pub async fn statuses(
    client: &Client,
    github_token: &str,
    branches: Vec<String>,
    commit: &str,
) -> Result<Vec<BranchStatus>, NixpkgsTrackError> {
    let checks = join_all(
        branches
            .iter()
            .map(|branch| contains(client, github_token, branch, commit)),
    )
    .await;

    branches
        .into_iter()
        .zip(checks)
        .map(|(name, contains)| {
            Ok(BranchStatus {
                name,
                contains: contains?,
            })
        })
        .collect()
}

pub fn reached(statuses: &[BranchStatus], branch: &str) -> bool {
    statuses.iter().any(|b| b.name == branch && b.contains)
}

/// One `branch: ✅/❌` line per branch.
pub fn describe(statuses: &[BranchStatus]) -> String {
    let mut description = String::new();
    for branch in statuses {
        let mark = if branch.contains { "✅" } else { "❌" };
        let _ = writeln!(description, "{}: {mark}", branch.name);
    }
    description
}

/// The stable release version (e.g. `24.11`) targeted by `base_ref`, if any.
fn stable_version(base_ref: &str) -> Option<String> {
    if ROLLING_BRANCHES.contains(&base_ref) {
        return None;
    }

    STABLE_VERSION_RE
        .find(base_ref)
        .map(|m| m.as_str().to_string())
}

fn tracked_branches_for(base_ref: &str) -> Vec<String> {
    if let Some(version) = stable_version(base_ref) {
        return STABLE_BRANCHES_TEMPLATE
            .iter()
            .map(|s| s.replace("XX.XX", &version))
            .collect();
    }

    ROLLING_BRANCHES.iter().map(|s| (*s).to_string()).collect()
}

/// `nixpkgs_track_lib` takes its client as `impl AsRef<Client>`.
struct W(Client);

impl AsRef<Client> for W {
    fn as_ref(&self) -> &Client {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pr(base_ref: &str) -> PullRequest {
        PullRequest {
            number: 1,
            title: "hello: 1.0 -> 2.0".to_string(),
            url: String::new(),
            base_ref: base_ref.to_string(),
            merge: Merge::NotMerged,
        }
    }

    #[test]
    fn open_prs_are_not_merged_even_with_a_test_merge_commit() {
        assert_eq!(
            Merge::from_github(false, Some("abc".to_string())),
            Merge::NotMerged
        );
        assert_eq!(
            Merge::from_github(true, Some("abc".to_string())),
            Merge::Merged("abc".to_string())
        );
        assert_eq!(Merge::from_github(true, None), Merge::NoMergeCommit);
    }

    #[test]
    fn stable_prs_track_release_branches() {
        assert_eq!(tracked_branches_for("release-24.11")[2], "release-24.11");
        assert_eq!(tracked_branches_for("master")[2], "master");
        assert_eq!(tracked_branches_for("staging-next")[3], "nixpkgs-unstable");
    }

    #[test]
    fn default_target_depends_on_base() {
        assert_eq!(pr("master").target_branch(None), "nixpkgs-unstable");
        assert_eq!(pr("release-24.11").target_branch(None), "release-24.11");
        assert_eq!(
            pr("master").target_branch(Some(TargetBranch::Unstable)),
            "nixos-unstable"
        );
    }

    #[test]
    fn reached_and_description() {
        let statuses: Vec<BranchStatus> = tracked_branches_for("master")
            .into_iter()
            .map(|name| BranchStatus {
                contains: name == "staging" || name == "master",
                name,
            })
            .collect();
        assert!(reached(&statuses, "master"));
        assert!(!reached(&statuses, "nixos-unstable"));
        assert!(describe(&statuses).starts_with("staging: ✅\nstaging-next: ❌\nmaster: ✅\n"));
    }
}
