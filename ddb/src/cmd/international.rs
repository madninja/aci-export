use super::{Result, connect_from_env, print_json};

/// International organization commands
///
/// Examples:
///   # Get international leadership
///   aci-ddb international leadership
#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: InternationalCmd,
}

impl Cmd {
    pub async fn run(&self) -> Result {
        self.cmd.run().await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum InternationalCmd {
    Leadership(LeadershipCmd),
}

#[derive(Debug, clap::Args)]
pub(crate) struct LeadershipCmd {}

impl InternationalCmd {
    pub async fn run(&self) -> Result {
        match self {
            Self::Leadership(_) => Leadership.run().await,
        }
    }
}

struct Leadership;

impl Leadership {
    pub async fn run(&self) -> Result {
        let db = connect_from_env().await?;
        let leadership = aci_ddb::leadership::for_international(&db).await?;
        print_json(&leadership)
    }
}
