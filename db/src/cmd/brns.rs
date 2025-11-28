use super::{connect_from_env, print_json, Result};
use db::brn;

#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: BrnCmd,
}

impl Cmd {
    pub async fn run(&self) -> Result {
        self.cmd.run().await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum BrnCmd {
    Email(Email),
    Number(Number),
}

impl BrnCmd {
    pub async fn run(&self) -> Result {
        match self {
            Self::Email(cmd) => cmd.run().await,
            Self::Number(cmd) => cmd.run().await,
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
        let brns = brn::by_email(&db, &self.email).await?;
        print_json(&brns)
    }
}

#[derive(Debug, clap::Args)]
pub struct Number {
    pub number: String,
}

impl Number {
    pub async fn run(&self) -> Result {
        let db = connect_from_env().await?;
        let brn = brn::by_number(&db, &self.number).await?;
        print_json(&brn)
    }
}
