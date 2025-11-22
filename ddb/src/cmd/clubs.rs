use super::{connect_from_env, print_json, Result};
use aci_ddb::clubs;
use anyhow::anyhow;

#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: ClubCmd,
}

impl Cmd {
    pub async fn run(&self) -> Result {
        self.cmd.run().await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum ClubCmd {
    List(List),
    Number(Number),
    Uid(Uid),
}

impl ClubCmd {
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
    pub async fn run(&self) -> Result {
        let db = connect_from_env().await?;
        let club = clubs::by_uid(&db, self.uid)
            .await?
            .ok_or_else(|| anyhow!("Club {} not found", self.uid))?;

        print_json(&club)
    }
}

#[derive(Debug, clap::Args)]
pub struct List {}

impl List {
    pub async fn run(&self) -> Result {
        let db = connect_from_env().await?;
        let clubs = clubs::all(&db).await?;
        print_json(&clubs)
    }
}
