use super::{client_from_env, print_json, Result};
use futures::TryStreamExt;

/// Commands on audience lists.
#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: ListsCommand,
}

impl Cmd {
    pub async fn run(&self) -> Result<()> {
        self.cmd.run().await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum ListsCommand {
    List(List),
    Create(Create),
    Delete(Delete),
    Info(Info),
    Update(Update),
}

impl ListsCommand {
    pub async fn run(&self) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.run().await,
            Self::Create(cmd) => cmd.run().await,
            Self::Delete(cmd) => cmd.run().await,
            Self::Info(cmd) => cmd.run().await,
            Self::Update(cmd) => cmd.run().await,
        }
    }
}

/// List all or a specific audience list.
#[derive(Debug, clap::Args)]
pub struct List {
    /// The list ID to get information for
    #[arg(long)]
    list: Option<String>,
}

impl List {
    pub async fn run(&self) -> Result<()> {
        let client = client_from_env()?;
        if let Some(list_id) = &self.list {
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
    /// Config file describing list to create
    descriptor: String,

    /// Merge fields to create
    merge_fields: Option<String>,
}

impl Create {
    pub async fn run(&self) -> Result<()> {
        let list = mailchimp::lists::List::from_config(config::File::with_name(&self.descriptor))?;
        let client = client_from_env()?;
        let new_list = mailchimp::lists::create(&client, &list).await?;

        if let Some(fields_descriptor) = &self.merge_fields {
            let merge_fields = mailchimp::merge_fields::MergeFields::from_config(
                config::File::with_name(fields_descriptor),
            )?;
            let _ =
                mailchimp::merge_fields::sync(&client, &new_list.id, merge_fields, true).await?;
        }

        print_json(&list)
    }
}

/// Delete an audience list.
#[derive(Debug, clap::Args)]
pub struct Delete {
    /// The list ID of the list to delete
    list: String,
}

impl Delete {
    pub async fn run(&self) -> Result<()> {
        let client = client_from_env()?;
        mailchimp::lists::delete(&client, &self.list).await?;
        Ok(())
    }
}

/// Get information about an audience list.
#[derive(Debug, clap::Args)]
pub struct Info {
    /// ID of the list to get
    list: String,
}

impl Info {
    pub async fn run(&self) -> Result<()> {
        let client = client_from_env()?;
        let info = mailchimp::lists::get(&client, &self.list).await?;
        print_json(&info)
    }
}

/// Udpate an audience to match a configuration file.
#[derive(Debug, clap::Args)]
pub struct Update {
    /// ID for list to update
    list: String,
    /// Descriptor file for list
    descriptor: String,
}

impl Update {
    pub async fn run(&self) -> Result<()> {
        let descriptor =
            mailchimp::lists::List::from_config(config::File::with_name(&self.descriptor))?;
        let client = client_from_env()?;
        let updated = mailchimp::lists::update(&client, &self.list, &descriptor).await?;

        print_json(&updated)
    }
}

