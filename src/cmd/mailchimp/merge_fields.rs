use crate::{cmd::print_json, settings::Settings, Result};
use futures::TryStreamExt;
use mailchimp::{self};

#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: MergeFieldsCommand,
}

impl Cmd {
    pub async fn run(&self, settings: &Settings) -> Result {
        self.cmd.run(settings).await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum MergeFieldsCommand {
    List(List),
    Create(Create),
    Delete(Delete),
}

impl MergeFieldsCommand {
    pub async fn run(&self, settings: &Settings) -> Result {
        match self {
            Self::List(cmd) => cmd.run(settings).await,
            Self::Create(cmd) => cmd.run(settings).await,
            Self::Delete(cmd) => cmd.run(settings).await,
        }
    }
}

/// List one or all the merge fields for a given audience list.
#[derive(Debug, clap::Args)]
pub struct List {
    /// The list ID to get merge fields for.
    list_id: String,
    /// The merge field ID of a specific field to get
    merge_id: Option<u32>,
}

impl List {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = mailchimp::client::from_api_key(&settings.mailchimp.api_key)?;
        if let Some(merge_id) = self.merge_id {
            let merge_field =
                mailchimp::merge_fields::get(&client, &self.list_id, merge_id).await?;
            print_json(&merge_field)
        } else {
            let lists = mailchimp::merge_fields::all(&client, &self.list_id, Default::default())
                .try_collect::<Vec<_>>()
                .await?;
            print_json(&lists)
        }
    }
}

/// Create a merge field for a given audience list.
#[derive(Debug, clap::Args)]
pub struct Create {
    /// The audience list ID.
    pub list_id: String,

    /// The type of the merge field.
    pub merge_type: mailchimp::merge_fields::MergeType,

    /// The tag for the merge field. Usually a short string that is used as a
    /// mail merge field.
    pub tag: String,

    /// The descriptive name of the merge field
    pub name: String,
}

impl Create {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = mailchimp::client::from_api_key(&settings.mailchimp.api_key)?;
        let merge_field = mailchimp::merge_fields::create(
            &client,
            &self.list_id,
            mailchimp::merge_fields::MergeField {
                tag: self.tag.clone(),
                name: self.name.clone(),
                r#type: self.merge_type.clone(),
                ..Default::default()
            },
        )
        .await?;
        print_json(&merge_field)
    }
}

#[derive(Debug, clap::Args)]
pub struct Delete {
    /// The audience list ID.
    pub list_id: String,

    /// The merge field ID.
    pub merge_id: String,
}

impl Delete {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = mailchimp::client::from_api_key(&settings.mailchimp.api_key)?;
        mailchimp::merge_fields::delete(&client, &self.list_id, &self.merge_id).await?;
        Ok(())
    }
}
