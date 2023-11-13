use crate::{
    settings::{MailchimpSetting, Settings},
    Result,
};

pub mod lists;
pub mod members;
pub mod merge_fields;
pub mod ping;

#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: MailchimpCommand,

    /// Mailchimp profile to use
    profile: String,
}

impl Cmd {
    pub async fn run(&self, settings: &Settings) -> Result {
        let profile = settings.profile(&self.profile)?;
        self.cmd.run(settings, profile).await
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
    pub async fn run(&self, settings: &Settings, profile: &MailchimpSetting) -> Result {
        match self {
            Self::Lists(cmd) => cmd.run(settings, profile).await,
            Self::Members(cmd) => cmd.run(settings, profile).await,
            Self::MergeFields(cmd) => cmd.run(settings, profile).await,
            Self::Ping(cmd) => cmd.run(settings, profile).await,
        }
    }
}
