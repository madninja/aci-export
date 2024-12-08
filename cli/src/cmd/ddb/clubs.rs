use crate::{cmd::print_json, settings::Settings, Result};
use anyhow::anyhow;
use ddb::clubs;

#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: ClubCmd,
}

impl Cmd {
    pub async fn run(&self, settings: &Settings) -> Result {
        self.cmd.run(settings).await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum ClubCmd {
    List(List),
    Number(Number),
    Uid(Uid),
}

impl ClubCmd {
    pub async fn run(&self, settings: &Settings) -> Result {
        match self {
            Self::Number(cmd) => cmd.run(settings).await,
            Self::Uid(cmd) => cmd.run(settings).await,
            Self::List(cmd) => cmd.run(settings).await,
        }
    }
}

#[derive(Debug, clap::Args)]
pub struct Number {
    pub number: i32,
}

impl Number {
    pub async fn run(&self, settings: &Settings) -> Result {
        let db = settings.database.connect().await?;
        let club = clubs::by_number(&db, self.number)
            .await?
            .ok_or_else(|| anyhow!("Club {} not found", self.number))?;

        print_json(&club)
    }
}

#[derive(Debug, clap::Args)]
pub struct Uid {
    pub uid: u64,
}

impl Uid {
    pub async fn run(&self, settings: &Settings) -> Result {
        let db = settings.database.connect().await?;
        let club = clubs::by_uid(&db, self.uid)
            .await?
            .ok_or_else(|| anyhow!("Club {} not found", self.uid))?;

        print_json(&club)
    }
}

#[derive(Debug, clap::Args)]
pub struct List {}

impl List {
    pub async fn run(&self, settings: &Settings) -> Result {
        let db = settings.database.connect().await?;
        let clubs = clubs::all(&db).await?;
        print_json(&clubs)
    }
}
