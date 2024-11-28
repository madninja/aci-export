use crate::{cmd::print_json, settings::Settings, Result};
use anyhow::anyhow;
use ddb::regions;
use futures::TryStreamExt;

#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: RegionCmd,
}

impl Cmd {
    pub async fn run(&self, settings: &Settings) -> Result {
        self.cmd.run(settings).await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum RegionCmd {
    List(List),
    Number(Number),
    Uid(Uid),
}

impl RegionCmd {
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
        let region = regions::by_number(&db, self.number)
            .await?
            .ok_or_else(|| anyhow!("Region {} not found", self.number))?;

        print_json(&region)
    }
}

#[derive(Debug, clap::Args)]
pub struct Uid {
    pub uid: u64,
}

impl Uid {
    pub async fn run(&self, settings: &Settings) -> Result {
        let db = settings.database.connect().await?;
        let region = regions::by_uid(&db, self.uid)
            .await?
            .ok_or_else(|| anyhow!("Region {} not found", self.uid))?;

        print_json(&region)
    }
}

#[derive(Debug, clap::Args)]
pub struct List {}

impl List {
    pub async fn run(&self, settings: &Settings) -> Result {
        let db = settings.database.connect().await?;
        let regions: Vec<regions::Region> = regions::all(&db).try_collect().await?;

        print_json(&regions)
    }
}
