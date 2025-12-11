use super::{Result, connect_from_env, print_json};
use aci_ddb::clubs;
use anyhow::anyhow;

/// Club management commands
///
/// Examples:
///   # List all clubs
///   aci-ddb clubs
///
///   # Get club by uid
///   aci-ddb clubs 12345
///
///   # Get club by number
///   aci-ddb clubs --number 42
///
///   # Get current leadership for all clubs
///   aci-ddb clubs leadership
///
///   # Get current leadership for club by uid
///   aci-ddb clubs leadership 12345
///
///   # Get current leadership for club by number
///   aci-ddb clubs leadership --number 42
///
///   # Get leadership as of a specific date
///   aci-ddb clubs leadership 2020-01-15
///   aci-ddb clubs leadership 12345 2020-01-15
///   aci-ddb clubs leadership --number 42 2020-01-15
#[derive(Debug, clap::Args)]
pub struct Cmd {
    /// Club uid or number (depending on --number flag). Omit to list all clubs.
    pub id: Option<u64>,

    /// Treat the id as a club number instead of uid
    #[arg(long)]
    pub number: bool,

    #[command(subcommand)]
    cmd: Option<ClubCmd>,
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
pub enum ClubCmd {
    Leadership(LeadershipCmd),
}

#[derive(Debug, clap::Args)]
pub(crate) struct LeadershipCmd {
    /// Optional club uid (default) or number (with --number flag). If not provided, returns leadership for all clubs.
    pub id: Option<u64>,

    /// Treat the id as a club number instead of uid
    #[arg(long)]
    pub number: bool,

    /// Optional date (YYYY-MM-DD) to get leadership as of that date. Omit for current leadership.
    pub as_of: Option<chrono::NaiveDate>,
}

impl ClubCmd {
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
    id: Option<u64>,
    number: bool,
}

impl Get {
    pub async fn run(&self) -> Result {
        let db = connect_from_env().await?;

        match (self.id, self.number) {
            (Some(id), true) => {
                // Lookup by number
                let club = clubs::by_number(&db, id as i32)
                    .await?
                    .ok_or_else(|| anyhow!("Club number {id} not found"))?;
                print_json(&club)
            }
            (Some(id), false) => {
                // Lookup by uid
                let club = clubs::by_uid(&db, id)
                    .await?
                    .ok_or_else(|| anyhow!("Club uid {id} not found"))?;
                print_json(&club)
            }
            (None, _) => {
                // No id - get all clubs
                let clubs = clubs::all(&db).await?;
                print_json(&clubs)
            }
        }
    }
}

struct Leadership {
    id: Option<u64>,
    number: bool,
    as_of: Option<chrono::NaiveDate>,
}

impl Leadership {
    pub async fn run(&self) -> Result {
        use aci_ddb::leadership::DateFilter;

        let db = connect_from_env().await?;
        let filter = self.as_of.map_or(DateFilter::Current, DateFilter::AsOf);

        let leadership = match (self.id, self.number) {
            (Some(id), true) => {
                aci_ddb::leadership::for_club_by_number(&db, id as i32, filter).await?
            }
            (Some(id), false) => aci_ddb::leadership::for_club(&db, id, filter).await?,
            (None, _) => aci_ddb::leadership::for_all_clubs(&db, filter).await?,
        };

        print_json(&leadership)
    }
}
