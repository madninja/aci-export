use crate::{cmd::print_json, settings::Settings, Result};
use futures::TryStreamExt;
use mailchimp::RetryPolicy;
use serde_json::json;
use std::sync::Arc;

/// Commands on audience lists.
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
    Info(Info),
    Update(Update),
    Sync(Sync),
}

impl ListsCommand {
    pub async fn run(&self, settings: &Settings) -> Result {
        match self {
            Self::List(cmd) => cmd.run(settings).await,
            Self::Create(cmd) => cmd.run(settings).await,
            Self::Delete(cmd) => cmd.run(settings).await,
            Self::Info(cmd) => cmd.run(settings).await,
            Self::Update(cmd) => cmd.run(settings).await,
            Self::Sync(cmd) => cmd.run(settings).await,
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
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = settings.mail.client()?;
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
    pub async fn run(&self, settings: &Settings) -> Result {
        let list = mailchimp::lists::List::from_config(config::File::with_name(&self.descriptor))?;
        let client = settings.mail.client()?;
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
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = settings.mail.client()?;
        mailchimp::lists::delete(&client, &self.list).await?;
        Ok(())
    }
}

/// Get information about an audience list.
#[derive(Debug, clap::Args)]
pub struct Info {
    /// ID of the list to get
    list: Option<String>,
}

impl Info {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = settings.mail.client()?;
        let list = settings.mail.list_override(&self.list)?;
        let info = mailchimp::lists::get(&client, list).await?;
        print_json(&info)
    }
}

/// Udpate an audience to match a configuration file.
#[derive(Debug, clap::Args)]
pub struct Update {
    /// ID for list to update
    list: Option<String>,
    /// Descriptor file for list
    descriptor: String,
}

impl Update {
    pub async fn run(&self, settings: &Settings) -> Result {
        let descriptor =
            mailchimp::lists::List::from_config(config::File::with_name(&self.descriptor))?;
        let list = settings.mail.list_override(&self.list)?;
        let client = settings.mail.client()?;
        let updated = mailchimp::lists::update(&client, list, &descriptor).await?;

        print_json(&updated)
    }
}

/// Sync a single emember, club members, or a members in a region to an audience
#[derive(Debug, clap::Args)]
pub struct Sync {
    /// List ID to sync settings with
    list: Option<String>,
    /// Merge fields dedescriptor file to use
    merge_fields: Option<String>,
    /// The email address of a single member to sync
    #[arg(long)]
    member: Option<String>,
    /// The ID of a club to sync
    #[arg(long)]
    club: Option<u64>,
    /// The ID of a region to sync
    #[arg(long)]
    region: Option<u64>,
}

impl Sync {
    pub async fn run(&self, settings: &Settings) -> Result {
        let merge_fields = mailchimp::merge_fields::MergeFields::from_config(
            config::File::with_name(settings.mail.fields_override(&self.merge_fields)?),
        )?;
        let list = settings.mail.list_override(&self.list)?;
        let client = Arc::new(settings.mail.client()?);
        let db = settings.database.connect().await?;

        let db_members = if let Some(email) = &self.member {
            vec![ddb::members::by_email(&db, email)
                .await?
                .ok_or(anyhow::anyhow!("Member not found: {email}"))?]
        } else if let Some(club) = settings.mail.club_override(self.club) {
            ddb::members::by_club(&db, club).await?
        } else if let Some(region) = settings.mail.region_override(self.region) {
            ddb::members::by_region(&db, region).await?
        } else {
            ddb::members::all(&db).await?
        };

        // Fetch addresses for primary members
        let db_addresses =
            ddb::members::mailing_address::for_members(&db, db_members.iter()).await?;

        // Convert ddb members to mailchimp members while injecting address
        let mc_members = ddb::members::mailchimp::to_members_with_address(
            &db_members,
            &db_addresses,
            &merge_fields,
        )
        .await?;

        let upserted = mailchimp::members::upsert_many(
            &client,
            list,
            futures::stream::iter(mc_members),
            RetryPolicy::with_retries(3),
        )
        .await?;

        let deleted = if self.member.is_none() {
            mailchimp::members::retain(&client, list, &upserted).await?
        } else {
            0
        };

        let tag_updates = ddb::members::mailchimp::to_tag_updates(&db_members);
        mailchimp::members::tags::update_many(
            &client,
            list,
            &tag_updates,
            RetryPolicy::with_retries(3),
        )
        .await?;

        let json = json!({
                "upserted": upserted.len(),
                "deleted": deleted,
        });
        print_json(&json)
    }
}
