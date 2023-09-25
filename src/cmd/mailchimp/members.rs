use crate::{cmd::print_json, settings::Settings, Result};
use futures::TryStreamExt;
use mailchimp::{self};

/// Get all or just one member of a given audience list.
#[derive(Debug, clap::Args)]
pub struct Cmd {
    /// List ID to get members for.
    list_id: String,
    /// Specific member email to get.
    member_email: Option<String>,
}

impl Cmd {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = mailchimp::client::from_api_key(&settings.mailchimp.api_key)?;
        if let Some(email) = &self.member_email {
            let member = mailchimp::members::for_email(&client, &self.list_id, email).await?;
            print_json(&member)
        } else {
            let lists = mailchimp::members::all(&client, &self.list_id, Default::default())
                .try_collect::<Vec<_>>()
                .await?;
            print_json(&lists)
        }
    }
}
