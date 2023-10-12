use crate::{cmd::print_json, settings::Settings, Result};
use futures::TryStreamExt;
use mailchimp::{self};

/// Commands on the members of an audience list.
#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: MembersCommand,
}

impl Cmd {
    pub async fn run(&self, settings: &Settings) -> Result {
        self.cmd.run(settings).await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum MembersCommand {
    List(List),
}

impl MembersCommand {
    pub async fn run(&self, settings: &Settings) -> Result {
        match self {
            Self::List(cmd) => cmd.run(settings).await,
            // Self::Sync(cmd) => cmd.run(settings).await,
        }
    }
}

/// Get all or just one member of a given audience list.
#[derive(Debug, clap::Args)]
pub struct List {
    /// List ID to get members for.
    list_id: String,
    /// Specific member email to get.
    member_email: Option<String>,
}

impl List {
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
