mod commands;
mod event_handler;

use dotenv::dotenv;
use reqwest::Client;
use std::env;

use color_eyre::eyre::{Report, Result};
use poise::serenity_prelude::{self as serenity, ActivityData, GatewayIntents};

#[derive(Debug)]
pub struct Data { // User data, which is stored and accessible in all command invocations
	client: Client,
} 

pub type Context<'a> = poise::Context<'a, Data, Report>;

#[tokio::main]
async fn main() -> Result<()> {
    // Load the .env file
    dotenv().ok();

    // Enable color_eyre beacuse error handling ig
    color_eyre::install()?;

    // Configure the client with your Discord bot token in the environment.
    let token = env::var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN to be set");
    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_PRESENCES
        | GatewayIntents::GUILD_MEMBERS;

    let opts = poise::FrameworkOptions {
        commands: vec![
            // user commands
            commands::user::whois::whois(),
            commands::user::avatar::avatar(),
            // bot commands
            commands::bot::ping::ping(),
            commands::bot::bot::botinfo(),
            // misc commands
            commands::misc::nixpkgs::nixpkgs(),
            // fun commands
            commands::fun::nix::nix(),
            commands::fun::chance::roll(),
            commands::fun::chance::raffle(),
            commands::fun::kittysay::kittysay(),
            commands::fun::bottom::topify(),
            commands::fun::bottom::bottomify(),
        ],
        event_handler: |ctx, event, _, data| Box::pin(crate::event_handler::event_handler(ctx, event, data)),
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                let commands =
                    poise::builtins::create_application_commands(&framework.options().commands);

                let guild_id = env::var("GUILD_ID")
                    .expect("Expected GUILD_ID to be set")
                    .parse::<u64>()?;

                ctx.set_activity(Some(ActivityData::custom("new bot, who dis?")));

                serenity::GuildId::new(guild_id)
                    .set_commands(ctx, commands)
                    .await?;

                Ok(Data {
					client: Client::builder()
						.user_agent("blahaj")
						.build()?,
				})
            })
        })
        .options(opts)
        .build();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client
        .expect("failed to find secrets")
        .start()
        .await
        .expect("failed to start client");
    Ok(())
}
