use crate::{cmd::print_json, settings::Settings, Result};
use futures::TryStreamExt;
use mailchimp::merge_fields::MergeField;
use serde_json::json;

/// Commands on the merge fields of an audience list.
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
    Sync(Sync),
}

impl MergeFieldsCommand {
    pub async fn run(&self, settings: &Settings) -> Result {
        match self {
            Self::List(cmd) => cmd.run(settings).await,
            Self::Create(cmd) => cmd.run(settings).await,
            Self::Delete(cmd) => cmd.run(settings).await,
            Self::Sync(cmd) => cmd.run(settings).await,
        }
    }
}

/// List one or all the merge fields for a given audience list.
#[derive(Debug, clap::Args)]
pub struct List {
    /// Override thelist ID to get merge fields for.
    #[arg(long)]
    list: Option<String>,
    /// The merge field ID of a specific field to get
    #[arg(long)]
    id: Option<u32>,
}

impl List {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = settings.mail.client()?;
        let list = settings.mail.list_override(&self.list)?;
        if let Some(merge_id) = self.id {
            let merge_field = mailchimp::merge_fields::get(&client, list, merge_id).await?;
            print_json(&merge_field)
        } else {
            let lists = mailchimp::merge_fields::all(&client, list, Default::default())
                .try_collect::<Vec<_>>()
                .await?;
            print_json(&lists)
        }
    }
}

/// Create a merge field for a given audience list.
#[derive(Debug, clap::Args)]
pub struct Create {
    /// Override the audience list ID.
    #[arg(long)]
    list: Option<String>,
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
        let client = settings.mail.client()?;
        let merge_field = mailchimp::merge_fields::create(
            &client,
            settings.mail.list_override(&self.list)?,
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

#[derive(Debug, clap::Args)]
pub struct Delete {
    /// Override the audience list ID.
    #[arg(long)]
    list: Option<String>,
    /// The merge field ID.
    pub merge_id: String,
}

impl Delete {
    pub async fn run(&self, settings: &Settings) -> Result {
        mailchimp::merge_fields::delete(
            &settings.mail.client()?,
            settings.mail.list_override(&self.list)?,
            &self.merge_id,
        )
        .await?;
        Ok(())
    }
}

#[derive(Debug, clap::Args)]
pub struct Sync {
    /// Override the audience list ID.
    #[arg(long)]
    list: Option<String>,
    /// The merge field definition file to configure for the audience
    pub merge_fields: Option<String>,
    /// Delete merge fields that are not present in the target list
    #[arg(long, default_value_t)]
    delete: bool,
}

impl Sync {
    pub async fn run(&self, settings: &Settings) -> Result {
        let merge_fields = mailchimp::merge_fields::MergeFields::from_config(
            config::File::with_name(settings.mail.fields_override(&self.merge_fields)?),
        )?;
        let (added, deleted, updated) = mailchimp::merge_fields::sync(
            &settings.mail.client()?,
            settings.mail.list_override(&self.list)?,
            merge_fields,
            self.delete,
        )
        .await?;

        let json = json!({
            "added": added,
            "deleted": deleted,
            "updated": updated,
        });
        print_json(&json)
    }
}
