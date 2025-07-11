mod commands;
mod event_handler;
mod types;

use dotenv::dotenv;
use std::env;

use color_eyre::eyre::Result;
use poise::serenity_prelude::{ActivityData, ClientBuilder, GatewayIntents};

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
            commands::misc::crates::crates(),
            // moderation commands
            commands::moderation::ban::ban(),
            commands::moderation::kick::kick(),
            commands::moderation::timeout::timeout(),
            // fun commands
            commands::fun::nix::nix(),
            commands::fun::chance::roll(),
            commands::fun::chance::raffle(),
            commands::fun::kittysay::kittysay(),
            commands::fun::bottom::topify(),
            commands::fun::bottom::bottomify(),
            commands::fun::pet::pet(),
            commands::fun::height::height(),
        ],
        event_handler: |ctx, event, _, data| {
            Box::pin(crate::event_handler::event_handler(ctx, event, data))
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                ctx.set_activity(Some(ActivityData::custom("new bot, who dis?")));
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(types::Data::new())
            })
        })
        .options(opts)
        .build();

    let client = ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    client
        .expect("failed to find secrets")
        .start()
        .await
        .expect("failed to start client");
    Ok(())
}
