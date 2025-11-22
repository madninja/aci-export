use super::{connect_from_env, print_json, Result};
use aci_ddb::members;
use anyhow::anyhow;

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
    All(All),
}

impl MemberCmd {
    pub async fn run(&self) -> Result {
        match self {
            Self::Email(cmd) => cmd.run().await,
            Self::Uid(cmd) => cmd.run().await,
            Self::Club(cmd) => cmd.run().await,
            Self::All(cmd) => cmd.run().await,
        }
    }
}

/// Look up an active member by email
#[derive(Debug, clap::Args)]
pub struct Email {
    /// Email address to look up
    pub email: String,
}

impl Email {
    pub async fn run(&self) -> Result {
        let db = connect_from_env().await?;
        let member = members::by_email(&db, &self.email)
            .await?
            .ok_or_else(|| anyhow!("Member {} not found", self.email))?;

        print_json(&member)
    }
}

/// Look up an active member by database uid
#[derive(Debug, clap::Args)]
pub struct Uid {
    /// UID to look up
    pub uid: u64,
}

impl Uid {
    pub async fn run(&self) -> Result {
        let db = connect_from_env().await?;
        let member = members::by_uid(&db, self.uid)
            .await?
            .ok_or_else(|| anyhow!("Member {} not found", self.uid))?;

        print_json(&member)
    }
}

/// Look up all members for a given club
///
/// This includes affiliate and local club members
#[derive(Debug, clap::Args)]
pub struct Club {
    /// UID of clyb to look up
    pub uid: u64,
}

impl Club {
    pub async fn run(&self) -> Result {
        let db = connect_from_env().await?;
        let members = members::by_club(&db, self.uid).await?;

        print_json(&members)
    }
}

/// Look up all active members in the database
#[derive(Debug, clap::Args)]
pub struct All {}

impl All {
    pub async fn run(&self) -> Result {
        let db = connect_from_env().await?;
        let members = members::all(&db).await?;

        print_json(&members)
    }
}
