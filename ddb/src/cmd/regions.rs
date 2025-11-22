use super::{connect_from_env, print_json, Result};
use aci_ddb::regions;
use anyhow::anyhow;

#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: RegionCmd,
}

impl Cmd {
    pub async fn run(&self) -> Result {
        self.cmd.run().await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum RegionCmd {
    List(List),
    Number(Number),
    Uid(Uid),
}

impl RegionCmd {
    pub async fn run(&self) -> Result {
        match self {
            Self::Number(cmd) => cmd.run().await,
            Self::Uid(cmd) => cmd.run().await,
            Self::List(cmd) => cmd.run().await,
        }
    }
}

#[derive(Debug, clap::Args)]
pub struct Number {
    pub number: i32,
}

impl Number {
    pub async fn run(&self) -> Result {
        let db = connect_from_env().await?;
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
    pub async fn run(&self) -> Result {
        let db = connect_from_env().await?;
        let region = regions::by_uid(&db, self.uid)
            .await?
            .ok_or_else(|| anyhow!("Region {} not found", self.uid))?;

        print_json(&region)
    }
}

#[derive(Debug, clap::Args)]
pub struct List {}

impl List {
    pub async fn run(&self) -> Result {
        let db = connect_from_env().await?;
        let regions = regions::all(&db).await?;
        print_json(&regions)
    }
}
