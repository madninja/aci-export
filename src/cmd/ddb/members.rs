use crate::{cmd::print_json, settings::Settings, Result};
use anyhow::anyhow;
use ddb::members;

#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: MemberCmd,
}

impl Cmd {
    pub async fn run(&self, settings: &Settings) -> Result {
        self.cmd.run(settings).await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum MemberCmd {
    Email(Email),
    Uid(Uid),
    Club(Club),
}

impl MemberCmd {
    pub async fn run(&self, settings: &Settings) -> Result {
        match self {
            Self::Email(cmd) => cmd.run(settings).await,
            Self::Uid(cmd) => cmd.run(settings).await,
            Self::Club(cmd) => cmd.run(settings).await,
        }
    }
}

#[derive(Debug, clap::Args)]
pub struct Email {
    pub email: String,
}

impl Email {
    pub async fn run(&self, settings: &Settings) -> Result {
        let db = settings.database.connect().await?;
        let member = members::by_email(&db, &self.email)
            .await?
            .ok_or_else(|| anyhow!("Member {} not found", self.email))?;

        print_json(&member)
    }
}

#[derive(Debug, clap::Args)]
pub struct Uid {
    pub uid: u64,
}

impl Uid {
    pub async fn run(&self, settings: &Settings) -> Result {
        let db = settings.database.connect().await?;
        let member = members::by_uid(&db, self.uid)
            .await?
            .ok_or_else(|| anyhow!("Member {} not found", self.uid))?;

        print_json(&member)
    }
}

#[derive(Debug, clap::Args)]
pub struct Club {
    pub uid: u64,
}

impl Club {
    pub async fn run(&self, settings: &Settings) -> Result {
        let db = settings.database.connect().await?;
        let members = members::by_club(&db, self.uid).await?;

        print_json(&members)
    }
}
