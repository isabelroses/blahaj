mod avatarsync;
mod color_roles;
mod commands;
mod config;
mod db;
mod event_handler;
mod nixpkgs;
mod outbound;
mod starboard;
mod timezones;
mod types;

use dotenv::dotenv;

use color_eyre::eyre::Result;
use poise::serenity_prelude::{ActivityData, ClientBuilder, GatewayIntents};

#[tokio::main]
async fn main() -> Result<()> {
    // Load the .env file
    dotenv().ok();

    // Enable color_eyre beacuse error handling ig
    color_eyre::install()?;
    let config = config::init()?;
    let data = types::Data::new();
    nixpkgs::packages::ensure_database(&data.client).await?;

    // Configure the client with your Discord bot token in the environment.
    let token = config.discord_token.clone();

    let intents = GatewayIntents::non_privileged()
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MEMBERS;

    let opts = poise::FrameworkOptions {
        commands: commands::all(),
        event_handler: |ctx, event, _, data| {
            Box::pin(crate::event_handler::event_handler(ctx, event, data))
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                ctx.set_activity(Some(ActivityData::custom("meow meow meow")));

                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                nixpkgs::packages::spawn_refresher(data.client.clone());
                nixpkgs::tracking::spawn_poller(
                    ctx.clone(),
                    data.client.clone(),
                    data.github_token.clone(),
                );

                Ok(data)
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
