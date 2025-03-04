use clap::Parser;
use std::{env, process};
use sync_app::{cron::sync_db, server, settings::Settings, Result};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Parser)]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(name = env!("CARGO_BIN_NAME"))]
pub struct Cli {
    #[clap(subcommand)]
    cmd: Option<Cmd>,
}

impl Cli {
    async fn run(&self) -> Result {
        let settings = Settings::new()?;

        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::new(&settings.log))
            .with(tracing_subscriber::fmt::layer())
            .init();

        if let Some(cmd) = self.cmd.as_ref() {
            cmd.run(settings).await?;
        } else {
            server::run(settings).await?;
        }

        Ok(())
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    Sync,
}

impl Cmd {
    async fn run(&self, settings: Settings) -> Result {
        match self {
            Self::Sync => {
                let stats = sync_db::run(settings).await?;
                println!("{}", &serde_json::to_string_pretty(&stats)?);
                Ok(())
            }
        }
    }
}

#[tokio::main]
async fn main() -> Result {
    let cli = Cli::parse();
    if let Err(e) = cli.run().await {
        eprintln!("error: {:?}", e);
        process::exit(1);
    }

    Ok(())
}
