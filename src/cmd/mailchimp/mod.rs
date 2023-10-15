use crate::{settings::Settings, Result};

pub mod lists;
pub mod members;
pub mod merge_fields;
pub mod ping;

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
    Lists(lists::Cmd),
    Members(members::Cmd),
    MergeFields(merge_fields::Cmd),
    Ping(ping::Cmd),
}

impl MailchimpCommand {
    pub async fn run(&self, settings: &Settings) -> Result {
        match self {
            Self::Lists(cmd) => cmd.run(settings).await,
            Self::Members(cmd) => cmd.run(settings).await,
            Self::MergeFields(cmd) => cmd.run(settings).await,
            Self::Ping(cmd) => cmd.run(settings).await,
        }
    }
}

pub fn read_toml<'de, T: serde::Deserialize<'de>>(path: &str) -> Result<T> {
    let config = config::Config::builder()
        .add_source(config::File::with_name(path))
        .build()
        .and_then(|config| config.try_deserialize())?;
    Ok(config)
}
