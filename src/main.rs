mod commands;
mod event_handler;
mod http_server;
mod types;

use dotenv::dotenv;
use std::{env, sync::Arc};

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
            commands::user::color_me::color_me(),
            // bot commands
            commands::bot::ping::ping(),
            commands::bot::bot::botinfo(),
            // misc commands
            commands::misc::crates::crates(),
            // moderation commands
            commands::moderation::ban::ban(),
            commands::moderation::kick::kick(),
            commands::moderation::purge::purge(),
            commands::moderation::timeout::timeout(),
            // commands for nix
            commands::nix::nixpkgs::nixpkgs(),
            commands::nix::nix::nix(),
            commands::nix::nixpkg::nixpkg(),
            // fun commands
            commands::fun::chance::roll(),
            commands::fun::kittysay::kittysay(),
            commands::fun::bottom::topify(),
            commands::fun::bottom::bottomify(),
            commands::fun::pet::pet(),
            commands::fun::height::height(),
            commands::fun::they::they(),
            commands::fun::nixdle::nixdle(),
        ],
        event_handler: |ctx, event, _, data| {
            Box::pin(crate::event_handler::event_handler(ctx, event, data))
        },
        ..Default::default()
    };

    let framework = poise::Framework::builder()
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                ctx.set_activity(Some(ActivityData::custom("meow meow meow")));

                poise::builtins::register_globally(ctx, &framework.options().commands).await?;

                // h tee tee pee
                let ctx_clone = Arc::new(ctx.clone());
                let data_clone = Arc::new(types::Data::new().await);

                sqlx::migrate!("./migrations")
                    .run(&data_clone.db_pool)
                    .await
                    .expect("failed to run database migrations");

                tokio::spawn(async move {
                    if let Err(e) =
                        http_server::start_http_server(ctx_clone, data_clone, 3000).await
                    {
                        eprintln!("HTTP server error: {e}");
                    }
                });

                Ok(types::Data::new().await)
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
