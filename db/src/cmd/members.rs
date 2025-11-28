use super::{Result, connect_from_env, print_json};
use db::member;

#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: MemberCmd,
}

impl Cmd {
    pub async fn run(&self) -> Result {
        self.cmd.run().await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum MemberCmd {
    Email(Email),
    Uid(Uid),
    Club(Club),
    Region(Region),
    All(All),
}

impl MemberCmd {
    pub async fn run(&self) -> Result {
        match self {
            Self::Email(cmd) => cmd.run().await,
            Self::Uid(cmd) => cmd.run().await,
            Self::Club(cmd) => cmd.run().await,
            Self::Region(cmd) => cmd.run().await,
            Self::All(cmd) => cmd.run().await,
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
        let member = member::by_email(&db, &self.email).await?;
        print_json(&member)
    }
}

#[derive(Debug, clap::Args)]
pub struct Uid {
    pub uid: i64,
}

impl Uid {
    pub async fn run(&self) -> Result {
        let db = connect_from_env().await?;
        let member = member::by_uid(&db, self.uid).await?;
        print_json(&member)
    }
}

#[derive(Debug, clap::Args)]
pub struct Club {
    pub uid: i64,
}

impl Club {
    pub async fn run(&self) -> Result {
        let db = connect_from_env().await?;
        let members = member::by_club(&db, self.uid).await?;
        print_json(&members)
    }
}

#[derive(Debug, clap::Args)]
pub struct Region {
    pub uid: i64,
}

impl Region {
    pub async fn run(&self) -> Result {
        let db = connect_from_env().await?;
        let members = member::by_region(&db, self.uid).await?;
        print_json(&members)
    }
}

#[derive(Debug, clap::Args)]
pub struct All {}

impl All {
    pub async fn run(&self) -> Result {
        let db = connect_from_env().await?;
        let members = member::all(&db).await?;
        print_json(&members)
    }
}
