// the logic here is pretty much ripped from https://github.com/uncenter/discord-forum-bot/blob/main/src/modules/expandGitHubLinks.ts
// with some modifications so I can make it work on diffrent git hosts

use color_eyre::eyre::{self, Result};
use poise::serenity_prelude::{Context, FullEvent};
use regex::Regex;
use reqwest::Client;

pub async fn handle(ctx: &Context, event: &FullEvent) -> Result<()> {
    if let FullEvent::Message { new_message } = event {
        let code_blocks = extract_code_blocks(new_message.content.clone()).await?;

        if !code_blocks.is_empty() {
            new_message
                .channel_id
                .say(ctx, code_blocks.join("\n"))
                .await?;
        }
    }

    Ok(())
}

async fn extract_code_blocks(msg: String) -> Result<Vec<String>> {
    let re = Regex::new(
        r"https?://(?P<host>(git.*|codeberg\.org))/(?P<repo>[\w-]+/[\w.-]+)/(blob|(src/(commit|branch)))?/(?P<reference>\S+?)/(?P<file>\S+)#L(?P<start>\d+)(?:[~-]L?(?P<end>\d+)?)?",
    )?;

    let mut blocks: Vec<String> = Vec::new();
    let client = Client::new();

    for caps in re.captures_iter(&msg) {
        let (host, repo, reference, file, start, end) = extract_url_components(&caps)?;

        let raw_url = construct_raw_url(host, repo, reference, file);

        if let Ok(code_block) = fetch_code_block(&client, &raw_url, start, end, file).await {
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
        return Err(eyre::eyre!("Failed to fetch content from {}", raw_url));
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
        .last()
        .map_or("", remove_query_string)
        .to_lowercase();

    Ok(format_code_block(language, content))
}

fn format_code_block(language: String, content: String) -> String {
    if content.len() > 1950 {
        let truncated_content = content.lines().take(1950).collect::<Vec<&str>>().join("\n");
        format!(
            "```{}\n{}\n```\n... (lines not displayed)",
            language, truncated_content
        )
    } else {
        format!("```{}\n{}\n```", language, content)
    }
}

fn remove_query_string(input: &str) -> &str {
    input.split('?').next().unwrap_or(input)
}
