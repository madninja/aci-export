use crate::{cmd::print_json, settings::Settings, user::User, Result};

#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: UserCmd,
}

impl Cmd {
    pub async fn run(&self, settings: &Settings) -> Result {
        self.cmd.run(settings).await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum UserCmd {
    Email(Email),
    Uid(Uid),
}

impl UserCmd {
    pub async fn run(&self, settings: &Settings) -> Result {
        match self {
            Self::Email(cmd) => cmd.run(settings).await,
            Self::Uid(cmd) => cmd.run(settings).await,
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
        let user = User::by_email(&db, &self.email).await?;
        print_json(&user)
    }
}

#[derive(Debug, clap::Args)]
pub struct Uid {
    pub uid: u64,
}

impl Uid {
    pub async fn run(&self, settings: &Settings) -> Result {
        let db = settings.database.connect().await?;
        let user = User::by_uid(&db, self.uid).await?;
        print_json(&user)
    }
}
