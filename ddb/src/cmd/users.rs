use super::{Result, connect_from_env, print_json};
use aci_ddb::users;

#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: UserCmd,
}

impl Cmd {
    pub async fn run(&self) -> Result {
        self.cmd.run().await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum UserCmd {
    Email(Email),
    Uid(Uid),
}

impl UserCmd {
    pub async fn run(&self) -> Result {
        match self {
            Self::Email(cmd) => cmd.run().await,
            Self::Uid(cmd) => cmd.run().await,
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
        let user = users::by_email(&db, &self.email).await?;
        print_json(&user)
    }
}

#[derive(Debug, clap::Args)]
pub struct Uid {
    pub uid: u64,
}

impl Uid {
    pub async fn run(&self) -> Result {
        let db = connect_from_env().await?;
        let user = users::by_uid(&db, self.uid).await?;
        print_json(&user)
    }
}
