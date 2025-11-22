pub type Result<T = ()> = anyhow::Result<T>;

use anyhow::Context;

pub fn client_from_env() -> Result<mailchimp::Client> {
    let api_key = std::env::var("MAILCHIMP_API_KEY")
        .context("MAILCHIMP_API_KEY environment variable not set")?;
    Ok(mailchimp::client::from_api_key(&api_key)?)
}

pub mod lists;
pub mod members;
pub mod merge_fields;
pub mod ping;

pub fn print_json<T: ?Sized + serde::Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: MailchimpCommand,
}

impl Cmd {
    pub async fn run(&self) -> Result<()> {
        self.cmd.run().await
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
    pub async fn run(&self) -> Result<()> {
        match self {
            Self::Lists(cmd) => cmd.run().await,
            Self::Members(cmd) => cmd.run().await,
            Self::MergeFields(cmd) => cmd.run().await,
            Self::Ping(cmd) => cmd.run().await,
        }
    }
}
