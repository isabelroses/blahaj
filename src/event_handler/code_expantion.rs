// the logic here is pretty much ripped from https://github.com/uncenter/discord-forum-bot/blob/main/src/modules/expandGitHubLinks.ts
// with some modifications so I can make it work on diffrent git hosts

use std::sync::LazyLock;

use color_eyre::eyre::{Result, eyre};
use futures_util::future::join_all;
use poise::serenity_prelude::{Context, FullEvent};
use regex::Regex;
use reqwest::Client;

use crate::outbound::{self, Overflow};
use crate::types::Data;

static CODE_LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"https?://(?P<host>(git.*|codeberg\.org|tangled\.org))/(?P<repo>[\w-]+/[\w.-]+)/(blob|(src/(commit|branch)))?/(?P<reference>\S+?)/(?P<file>\S+)#L(?P<start>\d+)(?:[~-]L?(?P<end>\d+)?)?",
    )
    .unwrap()
});

/// Largest file we will download to expand a code link.
const MAX_FILE_BYTES: usize = 1024 * 1024;

pub async fn handle(ctx: &Context, event: &FullEvent, data: &Data) -> Result<()> {
    if let FullEvent::Message { new_message } = event {
        if !new_message.content.contains("://") {
            return Ok(());
        }

        let code_blocks = extract_code_blocks(&new_message.content, &data.client).await?;

        if !code_blocks.is_empty() {
            let attachment_name = attachment_name_for_code_blocks(&code_blocks);
            let response_text = code_blocks
                .iter()
                .map(|block| block.content.as_str())
                .collect::<Vec<&str>>()
                .join("\n");
            let response = outbound::fit(
                &response_text,
                Overflow::Attach {
                    file_name: &attachment_name,
                },
            );

            new_message.channel_id.send_message(ctx, response).await?;
        }
    }

    Ok(())
}

struct CodeBlock {
    content: String,
    file_name: String,
}

async fn extract_code_blocks(msg: &str, client: &Client) -> Result<Vec<CodeBlock>> {
    let mut requests = Vec::new();
    for caps in CODE_LINK_RE.captures_iter(msg) {
        let (host, repo, reference, file, start, end) = extract_url_components(&caps)?;
        let raw_url = construct_raw_url(host, repo, reference, file);
        requests.push((raw_url, start, end, file));
    }

    // Fetch concurrently; `join_all` keeps the message's order.
    let blocks = join_all(
        requests
            .iter()
            .map(|(raw_url, start, end, file)| async move {
                let content = fetch_code_block(client, raw_url, *start, *end, file)
                    .await
                    .ok()?;
                Some(CodeBlock {
                    content,
                    file_name: file_name(file).to_string(),
                })
            }),
    )
    .await;

    Ok(blocks.into_iter().flatten().collect())
}

/// Pulls the parts of a code link out of a regex match. The returned slices
/// borrow from the message text (`'h`) rather than from `caps`, so they outlive
/// each match.
fn extract_url_components<'h>(
    caps: &regex::Captures<'h>,
) -> Result<(&'h str, &'h str, &'h str, &'h str, usize, usize)> {
    let group = |name: &str| {
        caps.name(name)
            .map(|m| m.as_str())
            .ok_or_else(|| eyre!("code link is missing its {name}"))
    };

    let host = group("host")?;
    let repo = group("repo")?;
    let reference = group("reference")?;
    let file = group("file")?;
    let start = group("start")?.parse::<usize>()?;
    let end = caps
        .name("end")
        .map_or(Ok(start), |end| end.as_str().parse::<usize>())?;

    Ok((host, repo, reference, file, start, end))
}

fn construct_raw_url(host: &str, repo: &str, reference: &str, file: &str) -> String {
    if host == "github.com" {
        format!("https://raw.githubusercontent.com/{repo}/{reference}/{file}")
    } else {
        let refer = if reference.len() == 40 {
            format!("commit/{reference}")
        } else {
            format!("branch/{reference}")
        };
        format!("https://{host}/{repo}/raw/{refer}/{file}")
    }
}

async fn fetch_code_block(
    client: &Client,
    raw_url: &str,
    start: usize,
    end: usize,
    file: &str,
) -> Result<String> {
    if start == 0 || end < start {
        return Err(eyre!("Invalid line range L{start}-L{end}"));
    }

    let mut response = client.get(raw_url).send().await?;
    if !response.status().is_success() {
        return Err(eyre!("Failed to fetch content from {}", raw_url));
    }

    // Only a few lines are ever shown, so stop reading huge files early.
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        body.extend_from_slice(&chunk);
        if body.len() > MAX_FILE_BYTES {
            return Err(eyre!("{raw_url} is too large to expand"));
        }
    }

    let text = String::from_utf8_lossy(&body);
    let content = text
        .lines()
        .skip(start - 1)
        .take(end - start + 1)
        .collect::<Vec<&str>>()
        .join("\n");

    let language = file
        .split('.')
        .next_back()
        .map_or("", remove_query_string)
        .to_lowercase();

    Ok(format_code_block(&language, &content))
}

fn format_code_block(language: &str, content: &str) -> String {
    format!("```{language}\n{content}\n```")
}

fn attachment_name_for_code_blocks(code_blocks: &[CodeBlock]) -> String {
    if let [code_block] = code_blocks {
        code_block.file_name.clone()
    } else {
        "expanded-code.md".to_string()
    }
}

fn file_name(path: &str) -> &str {
    remove_query_string(path)
        .rsplit('/')
        .next()
        .unwrap_or("expanded-code.md")
}

fn remove_query_string(input: &str) -> &str {
    input.split('?').next().unwrap_or(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(link: &str) -> (String, usize, usize) {
        let caps = CODE_LINK_RE.captures(link).expect("link should match");
        let (host, repo, reference, file, start, end) = extract_url_components(&caps).unwrap();
        (construct_raw_url(host, repo, reference, file), start, end)
    }

    #[test]
    fn github_range_link() {
        assert_eq!(
            parse("see https://github.com/isabelroses/blahaj/blob/main/src/main.rs#L10-L20"),
            (
                "https://raw.githubusercontent.com/isabelroses/blahaj/main/src/main.rs".to_string(),
                10,
                20
            )
        );
    }

    #[test]
    fn single_line_link_ends_where_it_starts() {
        let (_, start, end) = parse("https://github.com/a/b/blob/main/x.rs#L7");
        assert_eq!((start, end), (7, 7));
    }

    #[test]
    fn non_github_host_uses_branch_raw_path() {
        let (url, _, _) = parse("https://codeberg.org/a/b/src/branch/main/x.rs#L1-3");
        assert_eq!(url, "https://codeberg.org/a/b/raw/branch/main/x.rs");
    }
}
