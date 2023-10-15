use crate::{settings::Settings, Result};

pub mod members;
pub mod users;

#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: DdbCommand,
}

impl Cmd {
    pub async fn run(&self, settings: &Settings) -> Result {
        self.cmd.run(settings).await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum DdbCommand {
    Users(users::Cmd),
    Members(members::Cmd),
}

impl DdbCommand {
    pub async fn run(&self, settings: &Settings) -> Result {
        match self {
            Self::Users(cmd) => cmd.run(settings).await,
            Self::Members(cmd) => cmd.run(settings).await,
        }
    }
}
