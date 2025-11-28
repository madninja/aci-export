use clap::Parser;
use sync_mail::{cmd::Cmd, settings::Settings, Result};

#[derive(Debug, Parser)]
#[command(name = "sync-mail")]
#[command(about = "Mailchimp sync CLI for ACI membership")]
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
