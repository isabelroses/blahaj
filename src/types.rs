use reqwest::Client;
use std::{convert::AsRef, env};

#[derive(Debug)]
// User data, which is stored and accessible in all command invocations
pub struct Data {
    pub client: Client,
    pub github_token: String,
}

impl Data {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("isabelroses/blahaj")
                .build()
                .unwrap(),
            github_token: env::var("GITHUB_TOKEN").expect("GITHUB_TOKEN not set"),
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
