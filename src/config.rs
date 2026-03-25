use color_eyre::eyre::{Result, eyre};
use confique::Config;
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Config, Debug, Clone)]
pub struct AppConfig {
    #[config(env = "DISCORD_TOKEN")]
    pub discord_token: String,

    #[config(env = "GITHUB_TOKEN")]
    pub github_token: String,

    #[config(env = "BLAHAJ_DATA_DIR", default = "/var/lib/blahaj")]
    pub data_dir: PathBuf,

    #[config(
        env = "NIXPKGS_CHANNEL",
        default = "https://channels.nixos.org/nixpkgs-unstable"
    )]
    pub nixpkgs_channel: String,
}

static CONFIG: OnceLock<AppConfig> = OnceLock::new();

pub fn init() -> Result<&'static AppConfig> {
    if let Some(config) = CONFIG.get() {
        return Ok(config);
    }

    let mut builder = AppConfig::builder().env();
    if let Ok(path) = std::env::var("BLAHAJ_CONFIG") {
        builder = builder.file(path);
    } else {
        builder = builder.file("blahaj.toml");
    }

    let config = builder.load().map_err(|err| eyre!("{err}"))?;
    CONFIG
        .set(config)
        .map_err(|_| eyre!("config already initialized"))?;
    Ok(CONFIG.get().expect("config initialized"))
}

pub fn get() -> &'static AppConfig {
    CONFIG.get().expect("config not initialized")
}
