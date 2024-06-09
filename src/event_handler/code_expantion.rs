use color_eyre::eyre::Result;
use poise::serenity_prelude::{self as serenity, Context, CreateMessage, FullEvent};
use regex::Regex;
use reqwest::Client;

pub async fn handle(ctx: &Context, event: &FullEvent) -> Result<()> {
    let pattern =
        r"https?:\/\/github\.com\/([\w-]+\/[\w.-]+)\/blob\/(.+?)\/(.+?)#L(\d+)[~-]?L?(\d*)";
    let re = Regex::new(pattern)?;

    if let FullEvent::Message { new_message } = event {
        let content = &new_message.content;
        if re.captures(content.as_str()).is_some() {
            let mut code_blocks = Vec::new();

            for caps in re.captures_iter(content) {
                let full_url = caps.get(0).unwrap().as_str();
                let repo = caps.get(1).unwrap().as_str();
                let reference = caps.get(2).unwrap().as_str();
                let file = caps.get(3).unwrap().as_str();
                let start_str = caps.get(4).unwrap().as_str();
                let start = start_str.parse::<usize>().unwrap_or(1);
                let end = if let Some(end_str) = caps.get(5) {
                    end_str.as_str().parse::<usize>().unwrap_or(start)
                } else {
                    start
                };

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
                            reference
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

                    let language = file.split('.').last().unwrap_or("").to_string();
                    code_blocks.push((name, language, content, body));
                }
            }

            code_blocks.retain(|(_, _, content, _)| !content.trim().is_empty());

            if !code_blocks.is_empty() {
                let embeds = code_blocks
                    .into_iter()
                    .map(|(name, language, content, body)| {
                        serenity::CreateEmbed::default()
                            .title(name)
                            .description(format!("```{}\n{}\n```", language, content).to_owned())
                            .footer(serenity::CreateEmbedFooter::new(body))
                    })
                    .collect::<Vec<_>>();

                new_message
                    .channel_id
                    .send_message(&ctx, CreateMessage::default().embeds(embeds))
                    .await?;
            }
        }
    }

    Ok(())
}
