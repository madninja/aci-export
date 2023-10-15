use aci_export::{
    cmd::{ddb, mailchimp},
    settings::Settings,
    Result,
};
use clap::Parser;
use std::{path::PathBuf, process};

#[derive(Debug, Parser)]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(name = env!("CARGO_BIN_NAME"))]
pub struct Cli {
    #[command(subcommand)]
    cmd: Cmd,

    /// Configuration file to use
    #[arg(short = 'c', default_value = "settings.toml")]
    config: PathBuf,
}

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    Ddb(ddb::Cmd),
    Mailchimp(mailchimp::Cmd),
}

#[tokio::main]
async fn main() -> Result {
    let cli = Cli::parse();
    if let Err(e) = run(cli).await {
        eprintln!("error: {:?}", e);
        process::exit(1);
    }

    Ok(())
}

async fn run(cli: Cli) -> Result {
    let settings = Settings::new(&cli.config)?;
    match cli.cmd {
        Cmd::Ddb(cmd) => cmd.run(&settings).await,
        Cmd::Mailchimp(cmd) => cmd.run(&settings).await,
    }
}
