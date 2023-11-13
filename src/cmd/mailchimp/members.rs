use crate::{
    cmd::print_json,
    settings::{MailchimpSetting, Settings},
    Result,
};
use futures::TryStreamExt;
use mailchimp::{self};

/// Commands on the members of an audience list.
#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: MembersCommand,
}

impl Cmd {
    pub async fn run(&self, settings: &Settings, profile: &MailchimpSetting) -> Result {
        self.cmd.run(settings, profile).await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum MembersCommand {
    List(List),
}

impl MembersCommand {
    pub async fn run(&self, settings: &Settings, profile: &MailchimpSetting) -> Result {
        match self {
            Self::List(cmd) => cmd.run(settings, profile).await,
        }
    }
}

/// Get all or just one member of the configured or a given audience list.
#[derive(Debug, clap::Args)]
pub struct List {
    /// List ID to get members for.
    #[arg(long)]
    list: Option<String>,
    /// Specific member email to get.
    #[arg(long)]
    member: Option<String>,
}

impl List {
    pub async fn run(&self, _settings: &Settings, profile: &MailchimpSetting) -> Result {
        let list_id = profile.list_override(&self.list)?;
        let client = profile.client()?;
        if let Some(email) = &self.member {
            let member = mailchimp::members::for_email(&client, list_id, email).await?;
            print_json(&member)
        } else {
            let lists = mailchimp::members::all(&client, list_id, Default::default())
                .try_collect::<Vec<_>>()
                .await?;
            print_json(&lists)
        }
    }
}
