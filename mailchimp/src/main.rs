mod cmd;

use anyhow::Result;
use clap::Parser;
use std::process;

#[derive(Debug, Parser)]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(name = env!("CARGO_BIN_NAME"))]
pub struct Cli {
    #[command(flatten)]
    cmd: cmd::Cmd,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv()?;
    let cli = Cli::parse();
    if let Err(e) = cli.cmd.run().await {
        eprintln!("error: {e:?}");
        process::exit(1);
    }

    Ok(())
}
