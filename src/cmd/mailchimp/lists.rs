use crate::{
    cmd::{mailchimp::read_toml, print_json},
    settings::Settings,
    Result,
};
use futures::TryStreamExt;

#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: ListsCommand,
}

impl Cmd {
    pub async fn run(&self, settings: &Settings) -> Result {
        self.cmd.run(settings).await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum ListsCommand {
    List(List),
    Create(Create),
    Delete(Delete),
}

impl ListsCommand {
    pub async fn run(&self, settings: &Settings) -> Result {
        match self {
            Self::List(cmd) => cmd.run(settings).await,
            Self::Create(cmd) => cmd.run(settings).await,
            Self::Delete(cmd) => cmd.run(settings).await,
        }
    }
}

/// List all or a specific audience list.
#[derive(Debug, clap::Args)]
pub struct List {
    /// The list ID to get information for
    list_id: Option<String>,
}

impl List {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = mailchimp::client::from_api_key(&settings.mailchimp.api_key)?;
        if let Some(list_id) = &self.list_id {
            let list = mailchimp::lists::get(&client, list_id).await?;
            print_json(&list)
        } else {
            let lists = mailchimp::lists::all(&client, Default::default())
                .try_collect::<Vec<_>>()
                .await?;
            print_json(&lists)
        }
    }
}

/// Create a new audience list.
#[derive(Debug, clap::Args)]
pub struct Create {
    /// The name of a file with the list configuration
    config: String,
}

impl Create {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = mailchimp::client::from_api_key(&settings.mailchimp.api_key)?;
        let list_config: mailchimp::lists::List = read_toml(&self.config)?;
        let list = mailchimp::lists::create(&client, &list_config).await?;

        print_json(&list)
    }
}

/// Delete an audience list.
#[derive(Debug, clap::Args)]
pub struct Delete {
    /// The list ID of the list to delete
    list_id: String,
}

impl Delete {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = mailchimp::client::from_api_key(&settings.mailchimp.api_key)?;
        mailchimp::lists::delete(&client, &self.list_id).await?;
        Ok(())
    }
}
