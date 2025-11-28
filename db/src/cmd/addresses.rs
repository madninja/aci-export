use super::{connect_from_env, print_json, Result};
use db::address;

#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: AddressCmd,
}

impl Cmd {
    pub async fn run(&self) -> Result {
        self.cmd.run().await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum AddressCmd {
    Email(Email),
}

impl AddressCmd {
    pub async fn run(&self) -> Result {
        match self {
            Self::Email(cmd) => cmd.run().await,
        }
    }
}

#[derive(Debug, clap::Args)]
pub struct Email {
    pub email: String,
}

impl Email {
    pub async fn run(&self) -> Result {
        let db = connect_from_env().await?;
        let addresses = address::by_email(&db, &self.email).await?;
        print_json(&addresses)
    }
}
