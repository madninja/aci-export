use clap::Parser;
use futures::{StreamExt, TryFutureExt, TryStreamExt};
use serde_json::json;
use std::{env, process};
use sync_mailchimp::{
    cron::mailchimp::{Job as MailchimpJob, JobUpdate as MailchimpJobUpdate},
    server,
    settings::Settings,
    Result,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Parser)]
#[command(version = env!("CARGO_PKG_VERSION"))]
#[command(name = env!("CARGO_BIN_NAME"))]
pub struct Cli {
    #[clap(subcommand)]
    cmd: Option<Cmd>,
}

impl Cli {
    async fn run(&self) -> Result {
        let settings = Settings::new()?;

        tracing_subscriber::registry()
            .with(tracing_subscriber::EnvFilter::new(&settings.log))
            .with(tracing_subscriber::fmt::layer())
            .init();

        if let Some(cmd) = self.cmd.as_ref() {
            cmd.run(settings).await?;
        } else {
            server::run(settings).await?;
        }

        Ok(())
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    Sync(Sync),
}

impl Cmd {
    async fn run(&self, settings: Settings) -> Result {
        match self {
            Self::Sync(cmd) => cmd.run(settings).await,
        }
    }
}

#[derive(Debug, clap::Args)]
pub struct Sync {
    #[clap(subcommand)]
    cmd: SyncCmd,
}

impl Sync {
    async fn run(&self, settings: Settings) -> Result {
        self.cmd.run(settings).await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum SyncCmd {
    List(SyncList),
    Create(SyncCreate),
    Update(SyncUpdate),
    Delete(SyncDelete),
    Fields(SyncFields),
    Run(SyncRun),
}

impl SyncCmd {
    async fn run(&self, settings: Settings) -> Result {
        match self {
            Self::List(cmd) => cmd.run(settings).await,
            Self::Create(cmd) => cmd.run(settings).await,
            Self::Update(cmd) => cmd.run(settings).await,
            Self::Delete(cmd) => cmd.run(settings).await,
            Self::Fields(cmd) => cmd.run(settings).await,
            Self::Run(cmd) => cmd.run(settings).await,
        }
    }
}

/// List the configured sync jobs
#[derive(Debug, clap::Args)]
pub struct SyncList {}

impl SyncList {
    async fn run(&self, settings: Settings) -> Result {
        let db = settings.db.connect().await?;
        let jobs = MailchimpJob::all(&db).await?;
        print_json(&jobs)
    }
}

/// Create a new sync job
#[derive(Debug, clap::Args)]
pub struct SyncCreate {
    #[arg(long)]
    name: String,
    #[arg(long)]
    club: Option<i64>,
    #[arg(long)]
    region: Option<i32>,
    #[arg(long)]
    api_key: String,
    #[arg(long)]
    list: String,
}

impl SyncCreate {
    async fn run(&self, settings: Settings) -> Result {
        let to_create = MailchimpJob {
            name: self.name.clone(),
            club: self.club,
            list: self.list.clone(),
            api_key: self.api_key.clone(),
            region: self.region,
            ..Default::default()
        };
        let db = settings.db.connect().await?;
        let job = MailchimpJob::create(&db, &to_create).await?;
        print_json(&job)
    }
}

/// Update a sync job
#[derive(Debug, clap::Args)]
pub struct SyncUpdate {
    /// The id of the job to update
    id: u64,
    #[arg(long)]
    name: Option<String>,
    #[arg(long)]
    club: Option<i64>,
    #[arg(long)]
    region: Option<i32>,
    #[arg(long)]
    api_key: Option<String>,
    #[arg(long)]
    list: Option<String>,
}

impl From<&SyncUpdate> for MailchimpJobUpdate {
    fn from(value: &SyncUpdate) -> Self {
        Self {
            id: value.id as i64,
            name: value.name.clone(),
            club: value.club,
            region: value.region,
            api_key: value.api_key.clone(),
            list: value.list.clone(),
        }
    }
}

impl SyncUpdate {
    async fn run(&self, settings: Settings) -> Result {
        let update = MailchimpJobUpdate::from(self);
        let db = settings.db.connect().await?;
        let job = MailchimpJob::update(&db, &update).await?;
        print_json(&job)
    }
}

/// Delete a sync job
///
/// Without the confirm flag this just lists the job that would be deleted
#[derive(Debug, clap::Args)]
pub struct SyncDelete {
    id: i64,
    #[arg(long)]
    confirm: bool,
}

impl SyncDelete {
    async fn run(&self, settings: Settings) -> Result {
        let db = settings.db.connect().await?;
        let job = MailchimpJob::get(&db, self.id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("no such job"))?;
        if self.confirm {
            MailchimpJob::delete(&db, self.id).await?;
            print_json(&json!({ "deleted": "ok" }))
        } else {
            print_json(&job)
        }
    }
}

/// Sync the mailing list merge fields for a given club (or all) to mailchimp
#[derive(Debug, clap::Args)]
pub struct SyncFields {
    /// The id of the mailing list to sync
    id: Option<u64>,
    /// Delete user added merge fields
    #[arg(long)]
    process_deletes: bool,
}

impl SyncFields {
    async fn run(&self, settings: Settings) -> Result {
        #[derive(Debug, serde::Serialize)]
        struct JobResult {
            name: String,
            deleted: Vec<String>,
            added: Vec<String>,
            updated: Vec<String>,
        }
        let db = settings.db.connect().await?;
        let jobs = if let Some(id) = self.id {
            let job = MailchimpJob::get(&db, id as i64)
                .await?
                .ok_or_else(|| anyhow::anyhow!("sync job not found"))?;
            vec![job]
        } else {
            MailchimpJob::all(&db).await?
        };
        let results = futures::stream::iter(jobs)
            .map(|job| async move {
                let name = job.name.clone();
                job.sync_merge_fields(self.process_deletes)
                    .map_ok(|(added, deleted, updated)| {
                        (
                            job.id,
                            JobResult {
                                name,
                                deleted: deleted.clone(),
                                added: added.clone(),
                                updated: updated.clone(),
                            },
                        )
                    })
                    .await
            })
            .buffered(20)
            .try_collect::<Vec<(i64, JobResult)>>()
            .await?;
        let map: std::collections::HashMap<i64, JobResult> = results.into_iter().collect();
        print_json(&map)
    }
}

/// Sync the given club (or all) mailing list from the membership database
#[derive(Debug, clap::Args)]
pub struct SyncRun {
    /// The id of the mailing list to sync
    id: Option<u64>,
}

impl SyncRun {
    async fn run(&self, settings: Settings) -> Result {
        #[derive(Debug, serde::Serialize)]
        struct JobResult {
            name: String,
            deleted: usize,
            upserted: usize,
        }
        let db = settings.db.connect().await?;
        let jobs = if let Some(id) = self.id {
            let job = MailchimpJob::get(&db, id as i64)
                .await?
                .ok_or_else(|| anyhow::anyhow!("sync job not found"))?;
            vec![job]
        } else {
            MailchimpJob::all(&db).await?
        };
        let results = futures::stream::iter(jobs)
            .map(|job| {
                let ddb_settings = settings.ddb.clone();
                async move {
                    let name = job.name.clone();
                    job.sync(ddb_settings)
                        .map_ok(|(deleted, upserted)| {
                            (
                                job.id,
                                JobResult {
                                    name,
                                    deleted,
                                    upserted,
                                },
                            )
                        })
                        .await
                }
            })
            .buffered(20)
            .try_collect::<Vec<(i64, JobResult)>>()
            .await?;

        let map: std::collections::HashMap<i64, JobResult> = results.into_iter().collect();
        print_json(&map)
    }
}

pub fn print_json<T: ?Sized + serde::Serialize>(value: &T) -> Result {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

#[tokio::main]
async fn main() -> Result {
    let cli = Cli::parse();
    if let Err(e) = cli.run().await {
        eprintln!("error: {:?}", e);
        process::exit(1);
    }

    Ok(())
}
