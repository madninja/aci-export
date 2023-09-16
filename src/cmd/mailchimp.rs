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
    Ping(Ping),
    // List(List),
}

impl MailchimpCommand {
    pub async fn run(&self, settings: &Settings) -> Result {
        match self {
            Self::Lists(cmd) => cmd.run(settings).await,
            Self::Members(cmd) => cmd.run(settings).await,
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
        let lists = mailchimp::lists::all(&client)
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
        let lists = mailchimp::lists::members::all(&client, &self.list_id)
            .try_collect::<Vec<_>>()
            .await?;
        print_json(&lists)
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
