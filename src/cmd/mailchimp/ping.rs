use crate::{cmd::print_json, settings::Settings, Result};
use mailchimp::{self};

#[derive(Debug, clap::Args)]
pub struct Cmd {}

impl Cmd {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = mailchimp::client::from_api_key(&settings.mailchimp.api_key)?;
        let status = mailchimp::health::ping(&client).await?;
        print_json(&status)
    }
}
