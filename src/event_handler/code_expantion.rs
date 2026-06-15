// the logic here is pretty much ripped from https://github.com/uncenter/discord-forum-bot/blob/main/src/modules/expandGitHubLinks.ts
// with some modifications so I can make it work on diffrent git hosts

use std::sync::LazyLock;

use color_eyre::eyre::{Result, eyre};
use poise::serenity_prelude::{Context, FullEvent};
use regex::Regex;
use reqwest::Client;

use crate::types::Data;

static CODE_LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"https?://(?P<host>(git.*|codeberg\.org|tangled\.org))/(?P<repo>[\w-]+/[\w.-]+)/(blob|(src/(commit|branch)))?/(?P<reference>\S+?)/(?P<file>\S+)#L(?P<start>\d+)(?:[~-]L?(?P<end>\d+)?)?",
    )
    .unwrap()
});

pub async fn handle(ctx: &Context, event: &FullEvent, data: &Data) -> Result<()> {
    if let FullEvent::Message { new_message } = event {
        if !new_message.content.contains("://") {
            return Ok(());
        }

        let code_blocks = extract_code_blocks(&new_message.content, &data.client).await?;

        if !code_blocks.is_empty() {
            new_message
                .channel_id
                .say(ctx, code_blocks.join("\n"))
                .await?;
        }
    }

    Ok(())
}

async fn extract_code_blocks(msg: &str, client: &Client) -> Result<Vec<String>> {
    let mut blocks: Vec<String> = Vec::new();

    for caps in CODE_LINK_RE.captures_iter(msg) {
        let (host, repo, reference, file, start, end) = extract_url_components(&caps)?;

        let raw_url = construct_raw_url(host, repo, reference, file);

        if let Ok(code_block) = fetch_code_block(client, &raw_url, start, end, file).await {
            blocks.push(code_block);
        }
    }

    Ok(blocks)
}

fn extract_url_components<'a>(
    caps: &'a regex::Captures<'a>,
) -> Result<(&'a str, &'a str, &'a str, &'a str, usize, usize)> {
    let host = &caps["host"];
    let repo = &caps["repo"];
    let reference = &caps["reference"];
    let file = &caps["file"];
    let start = caps["start"].parse::<usize>()?;
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
    let response = client.get(raw_url).send().await?;
    if !response.status().is_success() {
        return Err(eyre!("Failed to fetch content from {}", raw_url));
    }

    let text = response.text().await?;
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
    if content.len() > 1950 {
        let truncated_content = content.chars().take(1950).collect::<String>();
        format!("```{language}\n{truncated_content}\n```\n... (lines not displayed)")
    } else {
        format!("```{language}\n{content}\n```")
    }
}

fn remove_query_string(input: &str) -> &str {
    input.split('?').next().unwrap_or(input)
}
