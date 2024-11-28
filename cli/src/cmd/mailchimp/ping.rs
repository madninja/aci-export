use crate::{cmd::print_json, settings::Settings, Result};

#[derive(Debug, clap::Args)]
pub struct Cmd {}

impl Cmd {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = settings.mailchimp.client()?;
        let status = mailchimp::health::ping(&client).await?;
        print_json(&status)
    }
}
