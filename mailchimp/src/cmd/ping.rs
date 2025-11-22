use super::{client_from_env, print_json, Result};

#[derive(Debug, clap::Args)]
pub struct Cmd {}

impl Cmd {
    pub async fn run(&self) -> Result<()> {
        let client = client_from_env()?;
        let status = mailchimp::health::ping(&client).await?;
        print_json(&status)
    }
}
