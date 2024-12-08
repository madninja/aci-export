use clap::Parser;
use serde_json::json;
use server::{cron::mailchimp::Job as MailchimpJob, settings::Settings, Result};
use std::{env, process};
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
            server::server::run(settings).await?;
        }

        Ok(())
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    Setup(Setup),
}

impl Cmd {
    async fn run(&self, settings: Settings) -> Result {
        match self {
            Self::Setup(cmd) => cmd.run(settings).await,
        }
    }
}

#[derive(Debug, clap::Args)]
pub struct Setup {
    #[clap(subcommand)]
    cmd: SetupCmd,
}

impl Setup {
    async fn run(&self, settings: Settings) -> Result {
        self.cmd.run(settings).await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum SetupCmd {
    List(SetupList),
    Add(SetupAdd),
    Update(SetupUpdate),
    Delete(SetupDelete),
    Sync(SetupSync),
}

impl SetupCmd {
    async fn run(&self, settings: Settings) -> Result {
        match self {
            Self::List(cmd) => cmd.run(settings).await,
            Self::Add(cmd) => cmd.run(settings).await,
            Self::Update(cmd) => cmd.run(settings).await,
            Self::Delete(cmd) => cmd.run(settings).await,
            Self::Sync(cmd) => cmd.run(settings).await,
        }
    }
}

#[derive(Debug, clap::Args)]
pub struct SetupList {}

impl SetupList {
    async fn run(&self, settings: Settings) -> Result {
        let db = settings.db.connect().await?;
        let jobs = MailchimpJob::all(&db).await?;
        print_json(&jobs)
    }
}

#[derive(Debug, clap::Args)]
pub struct SetupAdd {
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

impl SetupAdd {
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

#[derive(Debug, clap::Args)]
pub struct SetupUpdate {
    id: i64,
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

impl SetupUpdate {
    async fn run(&self, settings: Settings) -> Result {
        let to_update = MailchimpJob {
            id: self.id,
            name: self.name.clone(),
            club: self.club,
            list: self.list.clone(),
            api_key: self.api_key.clone(),
            region: self.region,
            ..Default::default()
        };
        let db = settings.db.connect().await?;
        let job = MailchimpJob::update(&db, &to_update).await?;
        print_json(&job)
    }
}

#[derive(Debug, clap::Args)]
pub struct SetupDelete {
    id: i64,
    #[arg(long)]
    confirm: bool,
}

impl SetupDelete {
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

/// Sync the mailing list merge fields to mailchimp
#[derive(Debug, clap::Args)]
pub struct SetupSync {
    /// The id of the mailing list to sync
    id: u64,
    /// Delete user added merge fields
    #[arg(long)]
    process_deletes: bool,
}

impl SetupSync {
    async fn run(&self, settings: Settings) -> Result {
        let db = settings.db.connect().await?;
        let job = MailchimpJob::get(&db, self.id as i64)
            .await?
            .ok_or_else(|| anyhow::anyhow!("mailchimp job not found"))?;
        let (added, deleted, updated) = job.sync_merge_fields(self.process_deletes).await?;
        let json = json!({
            "added": added,
            "deleted": deleted,
            "updated": updated,
        });
        print_json(&json)
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
