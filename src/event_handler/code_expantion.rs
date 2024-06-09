// the logic here is pretty much ripped from https://github.com/uncenter/discord-forum-bot/blob/main/src/modules/expandGitHubLinks.ts
// with some modifications so I can make it work on diffrent git hosts

use color_eyre::eyre::Result;
use poise::serenity_prelude::{self as serenity, Context, CreateMessage, FullEvent};
use regex::Regex;
use reqwest::Client;

pub async fn handle(ctx: &Context, event: &FullEvent) -> Result<()> {
    if let FullEvent::Message { new_message } = event {
        let mut embeds: Vec<serenity::CreateEmbed> = Vec::new();

        embeds.extend(github_compatable_embeds(new_message.content.to_owned()).await?);

        if !embeds.is_empty() {
            new_message
                .channel_id
                .send_message(&ctx, CreateMessage::default().embeds(embeds))
                .await?;
        }
    }

    Ok(())
}

async fn github_compatable_embeds(msg: String) -> Result<Vec<serenity::CreateEmbed>> {
    let re = Regex::new(
        r"https?://github\.com/(?P<repo>[\w-]+/[\w.-]+)/blob/(?P<reference>\S+?)/(?P<file>\S+)#L(?P<start>\d+)(?:[~-]L?(?P<end>\d+)?)?"
        ).unwrap();

    let mut embeds: Vec<serenity::CreateEmbed> = Vec::new();

    if re.captures(msg.as_str()).is_some() {
        let mut code_blocks = Vec::new();

        for caps in re.captures_iter(&msg) {
            let full_url = caps.get(0).unwrap().as_str();

            let repo = caps["repo"].to_owned();
            let reference = caps["reference"].to_owned();
            let file = caps["file"].to_owned();
            let language = file.split('.').last().unwrap_or("").to_string();

            let start = caps["start"].parse::<usize>().unwrap_or(1);
            let end = caps["end"].parse::<usize>().unwrap_or(start);

            let raw_url = format!(
                "https://raw.githubusercontent.com/{}/{}/{}",
                repo, reference, file
            );
            let response = Client::new().get(&raw_url).send().await;

            if let Ok(response) = response {
                if !response.status().is_success() {
                    println!("Failed to fetch {} contents.", full_url);
                    continue;
                }

                let text = response.text().await?;
                let mut content = text
                    .lines()
                    .skip(start - 1)
                    .take(end - start + 1)
                    .collect::<Vec<&str>>()
                    .join("\n");

                let mut lines_sliced = 0;

                while content.len() > 1950 {
                    let lines = content.lines().collect::<Vec<&str>>();
                    if lines.len() == 1 {
                        content = content.chars().take(1950).collect();
                        break;
                    }
                    content = lines[..lines.len() - 1].join("\n");
                    lines_sliced += 1;
                }
                let end = end - lines_sliced;

                let name = format!(
                    "{repo}@{} {file} L{start}{}",
                    if reference.len() == 40 {
                        &reference[..8]
                    } else {
                        &reference
                    },
                    if end > start {
                        format!("-{}", end)
                    } else {
                        "".to_string()
                    }
                );

                let body = if lines_sliced > 0 {
                    format!("... ({} lines not displayed)", lines_sliced)
                } else {
                    "".to_string()
                };

                code_blocks.push((name, language, content, body));
            }
        }

        code_blocks.retain(|(_, _, content, _)| !content.trim().is_empty());

        if !code_blocks.is_empty() {
            embeds.extend(generic_codeblocks_to_embed(code_blocks));
        }
    }

    Ok(embeds)
}

type CodeBlock = (String, String, String, String);

fn generic_codeblocks_to_embed(codeblocks: Vec<CodeBlock>) -> Vec<serenity::CreateEmbed> {
    codeblocks
        .into_iter()
        .map(|(name, language, content, body)| {
            serenity::CreateEmbed::default()
                .title(name)
                .description(format!("```{}\n{}\n```", language, content).to_owned())
                .footer(serenity::CreateEmbedFooter::new(body))
        })
        .collect::<Vec<_>>()
}
