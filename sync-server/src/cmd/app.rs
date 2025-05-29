use crate::cmd::print_json;
use sync_server::{cron, settings::Settings, Result};

#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[clap(subcommand)]
    cmd: AppCmd,
}

impl Cmd {
    pub async fn run(&self, settings: Settings) -> Result {
        self.cmd.run(settings).await
    }
}

#[derive(Debug, clap::Subcommand)]
enum AppCmd {
    Run(AppRun),
}

impl AppCmd {
    pub async fn run(&self, settings: Settings) -> Result {
        match self {
            Self::Run(cmd) => cmd.run(settings).await,
        }
    }
}

#[derive(Debug, clap::Args)]
struct AppRun {}

impl AppRun {
    pub async fn run(&self, settings: Settings) -> Result {
        let stats = cron::db::run(&settings.app, &settings.ddb).await?;
        print_json(&stats)
    }
}
