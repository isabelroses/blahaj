// the logic here is pretty much ripped from https://github.com/uncenter/discord-forum-bot/blob/main/src/modules/expandGitHubLinks.ts
// with some modifications so I can make it work on diffrent git hosts

use color_eyre::eyre::Result;
use poise::serenity_prelude::{self as serenity, Context, CreateMessage, FullEvent};
use regex::Regex;
use reqwest::Client;

pub async fn handle(ctx: &Context, event: &FullEvent) -> Result<()> {
    if let FullEvent::Message { new_message } = event {
        let mut embeds: Vec<serenity::CreateEmbed> = Vec::new();

        embeds.extend(github_compatable_embeds(new_message.content.clone()).await?);

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
        r"https?://(?P<host>(git.*|codeberg\.org))/(?P<repo>[\w-]+/[\w.-]+)/(blob|(src/(commit|branch)))?/(?P<reference>\S+?)/(?P<file>\S+)#L(?P<start>\d+)(?:[~-]L?(?P<end>\d+)?)?"
        ).unwrap();

    let mut embeds: Vec<serenity::CreateEmbed> = Vec::new();

    let mut code_blocks = Vec::new();
    for caps in re.captures_iter(&msg) {
        let host = &caps["host"];
        let repo = &caps["repo"];
        let reference = &caps["reference"];
        let file = &caps["file"];
        let language = file.split('.').last().unwrap_or("").to_string();

        let start = caps["start"].parse::<usize>().unwrap_or(1);
        let end = caps
            .name("end")
            .and_then(|end| end.as_str().parse::<usize>().ok())
            .unwrap_or(start);

        let raw_url = if host == "github.com" {
            format!("https://raw.githubusercontent.com/{repo}/{reference}/{file}")
        } else {
            let refer = if reference.len() == 40 {
                format!("commit/{reference}")
            } else {
                format!("branch/{reference}")
            };
            format!("https://{host}/{repo}/raw/{refer}/{file}")
        };

        let response = Client::new().get(&raw_url).send().await;

        if let Ok(response) = response {
            if !response.status().is_success() {
                println!("Failed to fetch content.");
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
                    format!("-{end}")
                } else {
                    String::new()
                }
            );

            let body = if lines_sliced > 0 {
                format!("... ({lines_sliced} lines not displayed)")
            } else {
                String::new()
            };

            code_blocks.push((name, language, content, body));
        }

        code_blocks.retain(|(_, _, content, _)| !content.trim().is_empty());

        if !code_blocks.is_empty() {
            embeds.extend(generic_codeblocks_to_embed(&code_blocks));
        }
    }

    Ok(embeds)
}

type CodeBlock = (String, String, String, String);

fn generic_codeblocks_to_embed(codeblocks: &[CodeBlock]) -> Vec<serenity::CreateEmbed> {
    codeblocks
        .iter()
        .map(|(name, language, content, body)| {
            serenity::CreateEmbed::default()
                .title(name)
                .description(format!("```{language}\n{content}\n```").to_owned())
                .footer(serenity::CreateEmbedFooter::new(body))
        })
        .collect::<Vec<_>>()
}
