use reqwest::Client;

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
            github_token: crate::config::get().github_token.clone(),
        }
    }
}

pub type Context<'a> = poise::Context<'a, Data, color_eyre::eyre::Report>;
