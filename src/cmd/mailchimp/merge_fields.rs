use crate::{
    cmd::print_json,
    settings::{read_merge_fields, MailchimpSetting, Settings},
    Result,
};
use futures::TryStreamExt;
use mailchimp::merge_fields::{MergeField, MergeFields};
use serde_json::json;

/// Commands on the merge fields of an audience list.
#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: MergeFieldsCommand,
}

impl Cmd {
    pub async fn run(&self, settings: &Settings, profile: &MailchimpSetting) -> Result {
        self.cmd.run(settings, profile).await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum MergeFieldsCommand {
    List(List),
    Create(Create),
    Delete(Delete),
    Update(Update),
}

impl MergeFieldsCommand {
    pub async fn run(&self, settings: &Settings, profile: &MailchimpSetting) -> Result {
        match self {
            Self::List(cmd) => cmd.run(settings, profile).await,
            Self::Create(cmd) => cmd.run(settings, profile).await,
            Self::Delete(cmd) => cmd.run(settings, profile).await,
            Self::Update(cmd) => cmd.run(settings, profile).await,
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
    pub async fn run(&self, _settings: &Settings, profile: &MailchimpSetting) -> Result {
        let client = profile.client()?;
        let list = profile.list_override(&self.list)?;
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
    pub async fn run(&self, _settings: &Settings, profile: &MailchimpSetting) -> Result {
        let client = profile.client()?;
        let merge_field = mailchimp::merge_fields::create(
            &client,
            profile.list_override(&self.list)?,
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
    pub async fn run(&self, _settings: &Settings, profile: &MailchimpSetting) -> Result {
        mailchimp::merge_fields::delete(
            &profile.client()?,
            profile.list_override(&self.list)?,
            &self.merge_id,
        )
        .await?;
        Ok(())
    }
}

#[derive(Debug, clap::Args)]
pub struct Update {
    /// Override the audience list ID.
    #[arg(long)]
    list: Option<String>,
    /// The merge field definition file to configure for the audience
    pub merge_fields: Option<String>,
    /// Delete merge fields that are not present in the target list
    #[arg(long, default_value_t)]
    delete: bool,
}

impl Update {
    pub async fn run(&self, _settings: &Settings, profile: &MailchimpSetting) -> Result {
        let merge_fields = read_merge_fields(profile.fields_override(&self.merge_fields)?)?;
        let (added, deleted, updated) = update_merge_fields(
            &profile.client()?,
            profile.list_override(&self.list)?,
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

pub async fn update_merge_fields(
    client: &mailchimp::Client,
    list_id: &str,
    target: MergeFields,
    process_deletes: bool,
) -> Result<(Vec<String>, Vec<String>, Vec<String>)> {
    type TaggedMergeField = (String, MergeField);

    let current: MergeFields = mailchimp::merge_fields::all(client, list_id, Default::default())
        .try_collect()
        .await?;

    fn collect_tags(fields: &[TaggedMergeField]) -> Vec<String> {
        fields
            .iter()
            .map(|(_, field)| field.tag.clone())
            .collect::<Vec<_>>()
    }

    let (to_delete, _): (Vec<TaggedMergeField>, Vec<TaggedMergeField>) = current
        .clone()
        .into_iter()
        .partition(|(key, _)| !target.contains_key(key));

    let (to_add, target_remaining): (Vec<TaggedMergeField>, Vec<TaggedMergeField>) = target
        .into_iter()
        .partition(|(key, _)| !current.contains_key(key));

    let deleted = collect_tags(&to_delete);
    if process_deletes {
        for (_, field) in to_delete {
            mailchimp::merge_fields::delete(client, list_id, &field.merge_id.to_string()).await?;
        }
    }

    let added = collect_tags(&to_add);
    for (_, field) in to_add {
        mailchimp::merge_fields::create(client, list_id, field).await?;
    }

    let mut updated = vec![];
    for (_, mut field) in target_remaining.into_iter() {
        let current = current.get(&field.tag).unwrap();
        field.merge_id = current.merge_id;
        if field != *current {
            updated.push(field.tag.clone());
            mailchimp::merge_fields::update(client, list_id, &current.merge_id.to_string(), field)
                .await?;
        }
    }
    Ok((added, deleted, updated))
}
