mod commands;
mod event_handler;
mod types;

use dotenv::dotenv;
use std::{env, path::Path};

use color_eyre::eyre::Result;
use poise::serenity_prelude::{
    ActivityData, ChannelId, ClientBuilder, CreateAttachment, CreateMessage, GatewayIntents,
};
use sha2::{Digest, Sha256};

#[derive(Debug)]
struct NixpkgsRelease {
    url: String,
    hash: String,
}

async fn get_latest_nixpkgs_release() -> Result<NixpkgsRelease> {
    let base_url = env::var("NIXPKGS_CHANNEL")
        .unwrap_or_else(|_| "https://channels.nixos.org/nixpkgs-unstable".to_string());

    let response = reqwest::get(&base_url).await?;
    let html = response.text().await?;

    let url_regex =
        regex::Regex::new(r"<a href='([^']+/packages\.json\.br)'>packages\.json\.br</a>")?;
    let hash_regex = regex::Regex::new(
        r"packages\.json\.br</a></td><td align='right'>\d+</td><td><tt>([a-f0-9]{64})</tt>",
    )?;

    let url = url_regex
        .captures(&html)
        .and_then(|cap| cap.get(1))
        .map(|m| {
            let path = m.as_str();
            if path.starts_with("http") {
                path.to_string()
            } else if path.starts_with('/') {
                format!("https://releases.nixos.org{path}")
            } else {
                format!("https://releases.nixos.org/{path}")
            }
        })
        .ok_or_else(|| color_eyre::eyre::eyre!("Could not find packages.json.br URL"))?;

    let hash = hash_regex
        .captures(&html)
        .and_then(|cap| cap.get(1))
        .map(|m| m.as_str().to_string())
        .ok_or_else(|| color_eyre::eyre::eyre!("Could not find packages.json.br hash"))?;

    Ok(NixpkgsRelease { url, hash })
}

fn get_stored_hash() -> Option<String> {
    let hash_path = env::var("NIXPKGS_HASH_FILE").unwrap_or_else(|_| "nixpkgs.hash".to_string());
    std::fs::read_to_string(hash_path).ok()
}

fn store_hash(hash: &str) -> Result<()> {
    let hash_path = env::var("NIXPKGS_HASH_FILE").unwrap_or_else(|_| "nixpkgs.hash".to_string());
    std::fs::write(hash_path, hash)?;
    Ok(())
}

#[allow(clippy::too_many_lines)]
async fn ensure_nixpkgs_database() -> Result<()> {
    let db_path = env::var("NIXPKGS_DB").unwrap_or_else(|_| "nixpkgs.db".to_string());

    println!("Checking for nixpkgs updates...");
    let release = get_latest_nixpkgs_release().await?;
    let stored_hash = get_stored_hash();

    if Path::new(&db_path).exists() && stored_hash.as_deref() == Some(&release.hash) {
        println!("nixpkgs database is up to date");
        return Ok(());
    }

    if Path::new(&db_path).exists() {
        println!("New nixpkgs release detected, updating database...");
        std::fs::remove_file(&db_path)?;
    } else {
        println!("nixpkgs database not found, building...");
    }

    println!("Downloading from {}...", release.url);
    let response = reqwest::get(&release.url).await?;
    let compressed = response.bytes().await?;

    let mut hasher = Sha256::new();
    hasher.update(&compressed);
    let computed_hash = format!("{:x}", hasher.finalize());

    if computed_hash != release.hash {
        return Err(color_eyre::eyre::eyre!(
            "Hash mismatch! Expected {}, got {}",
            release.hash,
            computed_hash
        ));
    }

    println!("Hash verified, decompressing...");
    let mut decompressed = Vec::new();
    let mut decoder = brotli::Decompressor::new(compressed.as_ref(), 4096);
    std::io::copy(&mut decoder, &mut decompressed)?;

    println!("Parsing JSON...");
    let json_data: serde_json::Value = serde_json::from_slice(&decompressed)?;

    let packages = json_data["packages"]
        .as_object()
        .ok_or_else(|| color_eyre::eyre::eyre!("Invalid packages.json format"))?;

    println!("Creating database with {} packages...", packages.len());

    let mut conn = rusqlite::Connection::open(&db_path)?;

    conn.execute(
        "CREATE TABLE packages (
            package_name TEXT PRIMARY KEY,
            pname TEXT,
            version TEXT,
            name TEXT,
            system TEXT,
            output_name TEXT,
            available INTEGER,
            broken INTEGER,
            description TEXT,
            homepage TEXT,
            insecure INTEGER,
            unfree INTEGER,
            unsupported INTEGER,
            position TEXT,
            long_description TEXT,
            main_program TEXT,
            license_spdx_id TEXT,
            license_full_name TEXT,
            license_free INTEGER,
            license_url TEXT
        )",
        [],
    )?;

    conn.execute(
        "CREATE TABLE maintainers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            package_name TEXT,
            name TEXT,
            email TEXT,
            github TEXT,
            github_id INTEGER,
            matrix TEXT,
            FOREIGN KEY (package_name) REFERENCES packages(package_name)
        )",
        [],
    )?;

    conn.execute(
        "CREATE INDEX idx_package_name ON packages(package_name)",
        [],
    )?;
    conn.execute("CREATE INDEX idx_pname ON packages(pname)", [])?;
    conn.execute(
        "CREATE INDEX idx_maintainers_package ON maintainers(package_name)",
        [],
    )?;

    let total = packages.len();
    let mut count = 0;
    let batch_size = 1000;

    let mut package_batch = Vec::new();
    let mut maintainer_batch = Vec::new();

    for (pkg_name, pkg_data) in packages {
        let meta = &pkg_data["meta"];
        let license_data = &meta["license"];

        let license_spdx = match license_data {
            serde_json::Value::Object(obj) => obj
                .get("spdxId")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string),
            serde_json::Value::Array(arr) => {
                let ids: Vec<&str> = arr
                    .iter()
                    .filter_map(|v| v.get("spdxId"))
                    .filter_map(|v| v.as_str())
                    .collect();
                if ids.is_empty() {
                    None
                } else {
                    Some(ids.join(", "))
                }
            }
            serde_json::Value::String(s) => Some(s.clone()),
            _ => None,
        };

        let homepage = meta.get("homepage").and_then(|h| match h {
            serde_json::Value::String(s) => Some(s.as_str()),
            serde_json::Value::Array(arr) => arr.first().and_then(|v| v.as_str()),
            _ => None,
        });

        package_batch.push((
            pkg_name.clone(),
            pkg_data
                .get("pname")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string),
            pkg_data
                .get("version")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string),
            pkg_data
                .get("name")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string),
            pkg_data
                .get("system")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string),
            pkg_data
                .get("outputName")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string),
            i32::from(
                meta.get("available")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            ),
            i32::from(
                meta.get("broken")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            ),
            meta.get("description")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string),
            homepage.map(std::string::ToString::to_string),
            i32::from(
                meta.get("insecure")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            ),
            i32::from(
                meta.get("unfree")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            ),
            i32::from(
                meta.get("unsupported")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            ),
            meta.get("position")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string),
            meta.get("longDescription")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string),
            meta.get("mainProgram")
                .and_then(|v| v.as_str())
                .map(std::string::ToString::to_string),
            license_spdx,
            None::<String>,
            0,
            None::<String>,
        ));

        if let Some(maintainers) = meta.get("maintainers").and_then(|v| v.as_array()) {
            for m in maintainers {
                if let Some(obj) = m.as_object() {
                    maintainer_batch.push((
                        pkg_name.clone(),
                        obj.get("name")
                            .and_then(|v| v.as_str())
                            .map(std::string::ToString::to_string),
                        obj.get("email")
                            .and_then(|v| v.as_str())
                            .map(std::string::ToString::to_string),
                        obj.get("github")
                            .and_then(|v| v.as_str())
                            .map(std::string::ToString::to_string),
                        obj.get("githubId").and_then(serde_json::Value::as_i64),
                        obj.get("matrix")
                            .and_then(|v| v.as_str())
                            .map(std::string::ToString::to_string),
                    ));
                }
            }
        }

        count += 1;

        if package_batch.len() >= batch_size {
            let tx = conn.transaction()?;
            {
                let mut stmt = tx.prepare_cached("INSERT INTO packages VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")?;
                for p in &package_batch {
                    stmt.execute(rusqlite::params![
                        p.0, p.1, p.2, p.3, p.4, p.5, p.6, p.7, p.8, p.9, p.10, p.11, p.12, p.13,
                        p.14, p.15, p.16, p.17, p.18, p.19
                    ])?;
                }
            }
            {
                let mut stmt = tx.prepare_cached("INSERT INTO maintainers (package_name, name, email, github, github_id, matrix) VALUES (?, ?, ?, ?, ?, ?)")?;
                for m in &maintainer_batch {
                    stmt.execute(rusqlite::params![m.0, m.1, m.2, m.3, m.4, m.5])?;
                }
            }
            tx.commit()?;

            #[allow(clippy::cast_precision_loss)]
            let progress = (f64::from(count) / total as f64) * 100.0;

            println!("Progress: {count}/{total} ({progress:.1}%)");
            package_batch.clear();
            maintainer_batch.clear();
        }
    }

    if !package_batch.is_empty() {
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare_cached("INSERT INTO packages VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")?;
            for p in &package_batch {
                stmt.execute(rusqlite::params![
                    p.0, p.1, p.2, p.3, p.4, p.5, p.6, p.7, p.8, p.9, p.10, p.11, p.12, p.13, p.14,
                    p.15, p.16, p.17, p.18, p.19
                ])?;
            }
        }
        {
            let mut stmt = tx.prepare_cached("INSERT INTO maintainers (package_name, name, email, github, github_id, matrix) VALUES (?, ?, ?, ?, ?, ?)")?;
            for m in &maintainer_batch {
                stmt.execute(rusqlite::params![m.0, m.1, m.2, m.3, m.4, m.5])?;
            }
        }
        tx.commit()?;
    }

    println!("Vacuuming...");
    conn.execute("VACUUM", [])?;

    store_hash(&release.hash)?;

    println!("Database created successfully: {db_path}");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load the .env file
    dotenv().ok();

    // Enable color_eyre beacuse error handling ig
    color_eyre::install()?;
    ensure_nixpkgs_database().await?;

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
                        if let Err(e) = ensure_nixpkgs_database().await {
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
