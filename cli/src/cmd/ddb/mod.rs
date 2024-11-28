use crate::{settings::Settings, Result};

pub mod clubs;
pub mod members;
pub mod regions;
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
    Clubs(clubs::Cmd),
    Regions(regions::Cmd),
}

impl DdbCommand {
    pub async fn run(&self, settings: &Settings) -> Result {
        match self {
            Self::Users(cmd) => cmd.run(settings).await,
            Self::Members(cmd) => cmd.run(settings).await,
            Self::Clubs(cmd) => cmd.run(settings).await,
            Self::Regions(cmd) => cmd.run(settings).await,
        }
    }
}
