use reqwest::Client;
use sqlx::Pool;
use std::{convert::AsRef, env};

#[derive(Debug)]
// User data, which is stored and accessible in all command invocations
pub struct Data {
    pub client: Client,
    pub github_token: String,
    pub db_pool: Pool<sqlx::Sqlite>,
}

impl Data {
    pub async fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("isabelroses/blahaj")
                .build()
                .unwrap(),
            github_token: env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not set"),
            db_pool: Pool::connect(&env::var("DATABASE_PATH").expect("DATABASE_PATH not set"))
                .await
                .unwrap(),
        }
    }
}

pub type Context<'a> = poise::Context<'a, Data, color_eyre::eyre::Report>;

// wrapper for reqwest::Client
pub struct W<T>(pub T);

impl AsRef<Client> for W<Client> {
    fn as_ref(&self) -> &reqwest::Client {
        &self.0
    }
}
