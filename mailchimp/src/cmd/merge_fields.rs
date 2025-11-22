use super::{client_from_env, print_json, Result};
use futures::TryStreamExt;
use mailchimp::merge_fields::MergeField;

/// Commands on the merge fields of an audience list.
#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: MergeFieldsCommand,
}

impl Cmd {
    pub async fn run(&self) -> Result<()> {
        self.cmd.run().await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum MergeFieldsCommand {
    List(List),
    Create(Create),
    Delete(Delete),
}

impl MergeFieldsCommand {
    pub async fn run(&self) -> Result<()> {
        match self {
            Self::List(cmd) => cmd.run().await,
            Self::Create(cmd) => cmd.run().await,
            Self::Delete(cmd) => cmd.run().await,
        }
    }
}

/// List one or all the merge fields for a given audience list.
#[derive(Debug, clap::Args)]
pub struct List {
    /// The list ID to get merge fields for.
    list: String,
    /// The merge field ID of a specific field to get
    #[arg(long)]
    id: Option<u32>,
}

impl List {
    pub async fn run(&self) -> Result<()> {
        let client = client_from_env()?;
        if let Some(merge_id) = self.id {
            let merge_field = mailchimp::merge_fields::get(&client, &self.list, merge_id).await?;
            print_json(&merge_field)
        } else {
            let lists = mailchimp::merge_fields::all(&client, &self.list, Default::default())
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
    list: String,
    /// The type of the merge field.
    pub merge_type: mailchimp::merge_fields::MergeType,
    /// The tag for the merge field. Usually a short string that is used as a
    /// mail merge field.
    pub tag: String,
    /// The descriptive name of the merge field
    pub name: String,
}

impl Create {
    pub async fn run(&self) -> Result<()> {
        let client = client_from_env()?;
        let merge_field = mailchimp::merge_fields::create(
            &client,
            &self.list,
            MergeField {
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

/// Delete a merge field from an audience list.
#[derive(Debug, clap::Args)]
pub struct Delete {
    /// The audience list ID.
    list: String,
    /// The merge field ID.
    pub merge_id: String,
}

impl Delete {
    pub async fn run(&self) -> Result<()> {
        mailchimp::merge_fields::delete(&client_from_env()?, &self.list, &self.merge_id).await?;
        Ok(())
    }
}
