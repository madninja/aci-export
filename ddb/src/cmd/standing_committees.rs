use super::{Result, connect_from_env, print_json};
use aci_ddb::standing_committees;
use anyhow::anyhow;

/// Standing committee management commands
///
/// Examples:
///   # List all standing committees
///   aci-ddb standing-committees
///
///   # Get standing committee by uid
///   aci-ddb standing-committees 12345
///
///   # Get current leadership for all standing committees
///   aci-ddb standing-committees leadership
///
///   # Get current leadership for standing committee by uid
///   aci-ddb standing-committees leadership 12345
///
///   # Get leadership as of a specific date
///   aci-ddb standing-committees leadership 2020-01-15
///   aci-ddb standing-committees leadership 12345 2020-01-15
#[derive(Debug, clap::Args)]
pub struct Cmd {
    /// Standing committee uid. Omit to list all standing committees.
    pub uid: Option<u64>,

    #[command(subcommand)]
    cmd: Option<StandingCommitteeCmd>,
}

impl Cmd {
    pub async fn run(&self) -> Result {
        match &self.cmd {
            Some(cmd) => cmd.run().await,
            None => Get { uid: self.uid }.run().await,
        }
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum StandingCommitteeCmd {
    Leadership(LeadershipCmd),
}

#[derive(Debug, clap::Args)]
pub(crate) struct LeadershipCmd {
    /// Optional standing committee uid. If not provided, returns leadership for all standing committees.
    pub uid: Option<u64>,

    /// Optional date (YYYY-MM-DD) to get leadership as of that date. Omit for current leadership.
    pub as_of: Option<chrono::NaiveDate>,
}

impl StandingCommitteeCmd {
    pub async fn run(&self) -> Result {
        match self {
            Self::Leadership(args) => {
                Leadership {
                    uid: args.uid,
                    as_of: args.as_of,
                }
                .run()
                .await
            }
        }
    }
}

struct Get {
    uid: Option<u64>,
}

impl Get {
    pub async fn run(&self) -> Result {
        let db = connect_from_env().await?;

        match self.uid {
            Some(uid) => {
                let committee = standing_committees::by_uid(&db, uid)
                    .await?
                    .ok_or_else(|| anyhow!("Standing committee uid {uid} not found"))?;
                print_json(&committee)
            }
            None => {
                let committees = standing_committees::all(&db).await?;
                print_json(&committees)
            }
        }
    }
}

struct Leadership {
    uid: Option<u64>,
    as_of: Option<chrono::NaiveDate>,
}

impl Leadership {
    pub async fn run(&self) -> Result {
        use aci_ddb::leadership::DateFilter;

        let db = connect_from_env().await?;
        let filter = self.as_of.map_or(DateFilter::Current, DateFilter::AsOf);

        let leadership = match self.uid {
            Some(uid) => aci_ddb::leadership::for_standing_committee(&db, uid, filter).await?,
            None => aci_ddb::leadership::for_all_standing_committees(&db, filter).await?,
        };

        print_json(&leadership)
    }
}
