use super::{Result, connect_from_env, print_json};

/// International organization commands
///
/// Examples:
///   # Get current international leadership
///   db international leadership
///
///   # Get international leadership as of a specific date
///   db international leadership 2020-01-15
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
pub(crate) struct LeadershipCmd {
    /// Optional date (YYYY-MM-DD) to get leadership as of that date. Omit for current leadership.
    pub as_of: Option<chrono::NaiveDate>,
}

impl InternationalCmd {
    pub async fn run(&self) -> Result {
        match self {
            Self::Leadership(args) => Leadership { as_of: args.as_of }.run().await,
        }
    }
}

struct Leadership {
    as_of: Option<chrono::NaiveDate>,
}

impl Leadership {
    pub async fn run(&self) -> Result {
        use db::leadership::DateFilter;

        let db = connect_from_env().await?;
        let filter = self.as_of.map_or(DateFilter::Current, DateFilter::AsOf);
        let leadership = db::leadership::all(&db, filter).await?;
        print_json(&leadership)
    }
}
