pub type Result<T = ()> = anyhow::Result<T>;

use anyhow::Context;
use sqlx::PgPool;

pub async fn connect_from_env() -> Result<PgPool> {
    let url =
        std::env::var("ACI__APP_DB_URL").context("ACI__APP_DB_URL environment variable not set")?;
    let pool = PgPool::connect(&url).await.context("opening database")?;
    Ok(pool)
}

pub mod addresses;
pub mod brns;
pub mod clubs;
pub mod members;
pub mod regions;
pub mod users;

pub fn print_json<T: ?Sized + serde::Serialize>(value: &T) -> Result {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: DbCommand,
}

impl Cmd {
    pub async fn run(&self) -> Result {
        self.cmd.run().await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum DbCommand {
    Users(users::Cmd),
    Members(members::Cmd),
    Clubs(clubs::Cmd),
    Regions(regions::Cmd),
    Addresses(addresses::Cmd),
    Brns(brns::Cmd),
}

impl DbCommand {
    pub async fn run(&self) -> Result {
        match self {
            Self::Users(cmd) => cmd.run().await,
            Self::Members(cmd) => cmd.run().await,
            Self::Clubs(cmd) => cmd.run().await,
            Self::Regions(cmd) => cmd.run().await,
            Self::Addresses(cmd) => cmd.run().await,
            Self::Brns(cmd) => cmd.run().await,
        }
    }
}
