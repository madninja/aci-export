use clap::Parser;
use sync_app::{Result, cmd::Cmd, settings::Settings};

#[derive(Debug, Parser)]
#[command(name = "sync-app")]
#[command(about = "App database sync CLI for ACI membership")]
struct Args {
    #[command(flatten)]
    cmd: Cmd,
}

#[tokio::main]
async fn main() -> Result {
    dotenvy::dotenv().ok();

    let settings = Settings::new()?;

    tracing_subscriber::fmt()
        .with_env_filter(&settings.log)
        .init();

    let args = Args::parse();
    args.cmd.run(settings).await
}
