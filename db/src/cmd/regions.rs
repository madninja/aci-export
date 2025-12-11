use super::{Result, connect_from_env, print_json};
use anyhow::anyhow;
use db::region;

/// Region management commands
///
/// Examples:
///   # List all regions
///   db regions
///
///   # Get region by uid
///   db regions 456
///
///   # Get region by number
///   db regions --number 5
///
///   # Get current leadership for all regions
///   db regions leadership
///
///   # Get current leadership for region by uid
///   db regions leadership 456
///
///   # Get current leadership for region by number
///   db regions leadership --number 5
///
///   # Get leadership as of a specific date
///   db regions leadership 2020-01-15
///   db regions leadership 456 2020-01-15
///   db regions leadership --number 5 2020-01-15
#[derive(Debug, clap::Args)]
pub struct Cmd {
    /// Region uid or number (depending on --number flag). Omit to list all regions.
    pub id: Option<i64>,

    /// Treat the id as a region number instead of uid
    #[arg(long)]
    pub number: bool,

    #[command(subcommand)]
    cmd: Option<RegionCmd>,
}

impl Cmd {
    pub async fn run(&self) -> Result {
        match &self.cmd {
            Some(cmd) => cmd.run().await,
            None => {
                Get {
                    id: self.id,
                    number: self.number,
                }
                .run()
                .await
            }
        }
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum RegionCmd {
    Leadership(LeadershipCmd),
}

#[derive(Debug, clap::Args)]
pub(crate) struct LeadershipCmd {
    /// Optional region uid (default) or number (with --number flag). If not provided, returns leadership for all regions.
    pub id: Option<i64>,

    /// Treat the id as a region number instead of uid
    #[arg(long)]
    pub number: bool,

    /// Optional date (YYYY-MM-DD) to get leadership as of that date. Omit for current leadership.
    pub as_of: Option<chrono::NaiveDate>,
}

impl RegionCmd {
    pub async fn run(&self) -> Result {
        match self {
            Self::Leadership(args) => {
                Leadership {
                    id: args.id,
                    number: args.number,
                    as_of: args.as_of,
                }
                .run()
                .await
            }
        }
    }
}

struct Get {
    id: Option<i64>,
    number: bool,
}

impl Get {
    pub async fn run(&self) -> Result {
        let db = connect_from_env().await?;

        match (self.id, self.number) {
            (Some(id), true) => {
                // Lookup by number
                let region = region::by_number(&db, id as i32)
                    .await?
                    .ok_or_else(|| anyhow!("Region number {id} not found"))?;
                print_json(&region)
            }
            (Some(id), false) => {
                // Lookup by uid
                let region = region::by_uid(&db, id)
                    .await?
                    .ok_or_else(|| anyhow!("Region uid {id} not found"))?;
                print_json(&region)
            }
            (None, _) => {
                // No id - get all regions
                let regions = region::all(&db).await?;
                print_json(&regions)
            }
        }
    }
}

struct Leadership {
    id: Option<i64>,
    number: bool,
    as_of: Option<chrono::NaiveDate>,
}

impl Leadership {
    pub async fn run(&self) -> Result {
        use db::leadership::DateFilter;

        let db = connect_from_env().await?;
        let filter = self.as_of.map_or(DateFilter::Current, DateFilter::AsOf);

        let leadership = match (self.id, self.number) {
            (Some(id), true) => region::leadership_by_number(&db, id as i32, filter).await?,
            (Some(id), false) => region::leadership_by_uid(&db, id, filter).await?,
            (None, _) => region::all_leadership(&db, filter).await?,
        };

        print_json(&leadership)
    }
}
