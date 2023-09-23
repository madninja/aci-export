use crate::{cmd::print_json, settings::Settings, Result};
use futures::TryStreamExt;
use mailchimp::{self};

#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: MailchimpCommand,
}

impl Cmd {
    pub async fn run(&self, settings: &Settings) -> Result {
        self.cmd.run(settings).await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum MailchimpCommand {
    Lists(Lists),
    Members(Members),
    MergeFields(MergeFields),
    Ping(Ping),
    // List(List),
}

impl MailchimpCommand {
    pub async fn run(&self, settings: &Settings) -> Result {
        match self {
            Self::Lists(cmd) => cmd.run(settings).await,
            Self::Members(cmd) => cmd.run(settings).await,
            Self::MergeFields(cmd) => cmd.run(settings).await,
            Self::Ping(cmd) => cmd.run(settings).await,
            // Self::List(cmd) => cmd.run(settings).await,
        }
    }
}

#[derive(Debug, clap::Args)]
pub struct Lists {}

impl Lists {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = mailchimp::client::from_api_key(&settings.mailchimp.api_key)?;
        let lists = mailchimp::lists::all(&client, Default::default())
            .try_collect::<Vec<_>>()
            .await?;
        print_json(&lists)
    }
}

#[derive(Debug, clap::Args)]
pub struct Members {
    list_id: String,
}

impl Members {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = mailchimp::client::from_api_key(&settings.mailchimp.api_key)?;
        let lists = mailchimp::lists::members::all(&client, &self.list_id, Default::default())
            .try_collect::<Vec<_>>()
            .await?;
        print_json(&lists)
    }
}

#[derive(Debug, clap::Args)]
pub struct MergeFields {
    #[command(subcommand)]
    cmd: MergeFieldsCommand,
}

impl MergeFields {
    pub async fn run(&self, settings: &Settings) -> Result {
        self.cmd.run(settings).await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum MergeFieldsCommand {
    List(MergeFieldsList),
    Create(MergeFieldsCreate),
    Delete(MergeFieldsDelete),
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
pub struct MergeFieldsList {
    /// The list ID to get merge fields for.
    list_id: String,
    /// The merge field ID of a specific field to get
    merge_id: Option<u32>,
}

impl MergeFieldsList {
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
pub struct MergeFieldsCreate {
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

impl MergeFieldsCreate {
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
pub struct MergeFieldsDelete {
    /// The audience list ID.
    pub list_id: String,

    /// The merge field ID.
    pub merge_id: String,
}

impl MergeFieldsDelete {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = mailchimp::client::from_api_key(&settings.mailchimp.api_key)?;
        mailchimp::merge_fields::delete(&client, &self.list_id, &self.merge_id).await?;
        Ok(())
    }
}

#[derive(Debug, clap::Args)]
pub struct Ping {}

impl Ping {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = mailchimp::client::from_api_key(&settings.mailchimp.api_key)?;
        let status = mailchimp::health::ping(&client).await?;
        print_json(&status)
    }
}

// #[derive(Debug, clap::Args)]
// pub struct List {
//     #[command(subcommand)]
//     cmd: ListCommand,
// }

// impl List {
//     pub async fn run(&self, settings: &Settings) -> Result {
//         self.cmd.run(settings).await
//     }
// }

// #[derive(Debug, clap::Subcommand)]
// pub enum ListCommand {
//     Info(ListInfo),
// }

// impl ListCommand {
//     pub async fn run(&self, settings: &Settings) -> Result {
//         match self {
//             Self::Info(cmd) => cmd.run(settings).await,
//         }
//     }
// }

// #[derive(Debug, clap::Args)]
// pub struct ListInfo {
//     id: String,
// }

// impl ListInfo {
//     pub async fn run(&self, settings: &Settings) -> Result {
//         let client = mailchimp::client::from_api_key(&settings.mailchimp.api_key);
//         let info = mailchimp::list::info(&client, &self.id).await?;
//         print_json(&info)
//     }
// }
