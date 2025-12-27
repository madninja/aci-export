use super::{Result, connect_from_env, print_json};
use anyhow::anyhow;
use db::standing_committee;

/// Standing committee management commands
///
/// Examples:
///   # List all standing committees
///   db standing-committees
///
///   # Get standing committee by uid
///   db standing-committees 12345
///
///   # Get current leadership for all standing committees
///   db standing-committees leadership
///
///   # Get current leadership for standing committee by uid
///   db standing-committees leadership 12345
///
///   # Get leadership as of a specific date
///   db standing-committees leadership 2020-01-15
///   db standing-committees leadership 12345 2020-01-15
#[derive(Debug, clap::Args)]
pub struct Cmd {
    /// Standing committee uid. Omit to list all standing committees.
    pub uid: Option<i64>,

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
    pub uid: Option<i64>,

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
    uid: Option<i64>,
}

impl Get {
    pub async fn run(&self) -> Result {
        let db = connect_from_env().await?;

        match self.uid {
            Some(uid) => {
                let committee = standing_committee::by_uid(&db, uid)
                    .await?
                    .ok_or_else(|| anyhow!("Standing committee uid {uid} not found"))?;
                print_json(&committee)
            }
            None => {
                let committees = standing_committee::all(&db).await?;
                print_json(&committees)
            }
        }
    }
}

struct Leadership {
    uid: Option<i64>,
    as_of: Option<chrono::NaiveDate>,
}

impl Leadership {
    pub async fn run(&self) -> Result {
        use db::leadership::DateFilter;

        let db = connect_from_env().await?;
        let filter = self.as_of.map_or(DateFilter::Current, DateFilter::AsOf);

        let leadership = match self.uid {
            Some(uid) => standing_committee::leadership_by_uid(&db, uid, filter).await?,
            None => standing_committee::all_leadership(&db, filter).await?,
        };

        print_json(&leadership)
    }
}
