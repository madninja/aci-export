use crate::{Result, settings::Settings};

pub mod migrate;
pub mod run;

pub fn print_json<T: ?Sized + serde::Serialize>(value: &T) -> Result {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[clap(subcommand)]
    cmd: SyncCmd,
}

impl Cmd {
    pub async fn run(&self, settings: Settings) -> Result {
        self.cmd.run(settings).await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum SyncCmd {
    Run(run::Cmd),
    Migrate(migrate::Cmd),
}

impl SyncCmd {
    async fn run(&self, settings: Settings) -> Result {
        match self {
            Self::Run(cmd) => cmd.run(settings).await,
            Self::Migrate(cmd) => cmd.run(settings).await,
        }
    }
}
