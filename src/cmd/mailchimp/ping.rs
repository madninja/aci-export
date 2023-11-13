use crate::{
    cmd::print_json,
    settings::{MailchimpSetting, Settings},
    Result,
};
use mailchimp::{self};

#[derive(Debug, clap::Args)]
pub struct Cmd {}

impl Cmd {
    pub async fn run(&self, _settings: &Settings, profile: &MailchimpSetting) -> Result {
        let client = profile.client()?;
        let status = mailchimp::health::ping(&client).await?;
        print_json(&status)
    }
}
