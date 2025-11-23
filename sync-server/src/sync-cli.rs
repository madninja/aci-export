use clap::Parser;
use std::{env, process};
use sync_server::{settings::Settings, Result};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod cmd;
use cmd::{app, mail};

#[derive(Debug, Parser)]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(name = env!("CARGO_BIN_NAME"))]
pub struct Cli {
    #[clap(subcommand)]
    cmd: Cmd,
}

impl Cli {
    async fn run(&self) -> Result {
        dotenvy::dotenv()?;
        let settings = Settings::new()?;
        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::new(&settings.log))
            .with(tracing_subscriber::fmt::layer())
            .init();

        self.cmd.run(settings).await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    Mail(mail::Cmd),
    App(app::Cmd),
    Run(Run),
}

/// Run both mail and app sync concurrently
#[derive(Debug, clap::Args)]
pub struct Run {}

impl Run {
    async fn run(&self, settings: Settings) -> Result {
        use serde_json::json;

        let mail_settings = settings.clone();
        let app_settings = settings.clone();

        let (mail_stats, app_stats) = tokio::try_join!(
            async {
                let db = mail_settings.mail.db.connect().await?;
                let jobs = sync_server::cron::mailchimp::Job::all(&db).await?;
                sync_server::cron::mailchimp::Job::sync_many(jobs, mail_settings.ddb).await
            },
            async { sync_server::cron::db::run(&app_settings.app, &app_settings.ddb).await }
        )?;

        let combined = json!({
            "mail": mail_stats,
            "app": app_stats,
        });

        cmd::print_json(&combined)
    }
}

impl Cmd {
    async fn run(&self, settings: Settings) -> Result {
        match self {
            Self::Mail(cmd) => cmd.run(settings).await,
            Self::App(cmd) => cmd.run(settings).await,
            Self::Run(cmd) => cmd.run(settings).await,
        }
    }
}

#[tokio::main]
async fn main() -> Result {
    let cli = Cli::parse();
    if let Err(e) = cli.run().await {
        eprintln!("error: {e:?}");
        process::exit(1);
    }

    Ok(())
}
