mod commands;
mod event_handler;
mod nixpkgs_db;
mod types;
mod utils;

use dotenv::dotenv;
use std::env;

use color_eyre::eyre::Result;
use poise::serenity_prelude::{
    ActivityData, ChannelId, ClientBuilder, CreateAttachment, CreateMessage, GatewayIntents,
};

#[tokio::main]
#[allow(clippy::too_many_lines)]
async fn main() -> Result<()> {
    // Load the .env file
    dotenv().ok();

    // Enable color_eyre beacuse error handling ig
    color_eyre::install()?;
    nixpkgs_db::ensure_nixpkgs_database().await?;

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
            commands::misc::starboard::starboard_enable(),
            commands::misc::starboard::starboard_disable(),
            commands::misc::starboard::starboard_config(),
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

                tokio::spawn(async {
                    let mut interval =
                        tokio::time::interval(std::time::Duration::from_secs(43_200));
                    loop {
                        interval.tick().await;
                        if let Err(e) = nixpkgs_db::ensure_nixpkgs_database().await {
                            eprintln!("Failed to update nixpkgs database: {e}");
                        }
                    }
                });

                let ctx_clone = ctx.clone();
                tokio::spawn(async move {
                    loop {
                        let now = chrono::Utc::now();
                        let target_time = now
                            .date_naive()
                            .and_hms_opt(11, 0, 0)
                            .expect("Invalid time");
                        let mut target_datetime =
                            chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(
                                target_time,
                                chrono::Utc,
                            );

                        if now.time()
                            >= chrono::NaiveTime::from_hms_opt(11, 0, 0).expect("Invalid time")
                        {
                            target_datetime += chrono::Duration::days(1);
                        }

                        let duration_until_target = (target_datetime - now)
                            .to_std()
                            .unwrap_or(std::time::Duration::from_secs(0));
                        tokio::time::sleep(duration_until_target).await;

                        let channel_id = ChannelId::new(1095083877380395202);
                        let attachment = CreateAttachment::path("assets/idk.webp").await;

                        match attachment {
                            Ok(file) => {
                                let builder = CreateMessage::new().add_file(file);
                                if let Err(e) = channel_id.send_message(&ctx_clone, builder).await {
                                    eprintln!("Failed to send daily idk.webp: {e}");
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to load assets/idk.webp: {e}");
                            }
                        }

                        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                    }
                });

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
