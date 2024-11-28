use crate::{cmd::print_json, settings::Settings, Result};
use futures::TryStreamExt;

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
        }
    }
}

/// Get all or just one member of a given audience list.
#[derive(Debug, clap::Args)]
pub struct List {
    /// List ID to get members for.
    #[arg(long)]
    list: Option<String>,
    /// Specific member email to get.
    #[arg(long)]
    email: Option<String>,
    /// Specific member id to get
    #[arg(long)]
    id: Option<String>,
}

impl List {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = settings.mailchimp.client()?;
        let list = settings.mailchimp.list_override(&self.list)?;
        if let Some(member_id) = &self.id {
            let member = mailchimp::members::for_id(&client, list, member_id).await?;
            print_json(&member)
        } else if let Some(email) = &self.email {
            let member = mailchimp::members::for_email(&client, list, email).await?;
            print_json(&member)
        } else {
            let lists = mailchimp::members::all(&client, list, Default::default())
                .try_collect::<Vec<_>>()
                .await?;
            print_json(&lists)
        }
    }
}
