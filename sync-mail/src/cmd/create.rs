use crate::{Result, cmd::print_json, mailchimp::Job, settings::Settings};

/// Create a new sync job
#[derive(Debug, clap::Args)]
pub struct Cmd {
    /// Name of the club
    #[arg(long)]
    name: String,
    /// Club or region to sync
    #[command(flatten)]
    club_or_region: ClubOrRegion,
    /// Mailchimp API key
    #[arg(long)]
    api_key: String,
    /// Mailchimp audience identifier
    #[arg(long)]
    list: String,
}

#[derive(Debug, clap::Args)]
#[group(required = true, multiple = false)]
struct ClubOrRegion {
    #[arg(long)]
    club: Option<i64>,
    #[arg(long)]
    region: Option<i32>,
}

impl Cmd {
    pub async fn run(&self, settings: Settings) -> Result {
        let to_create = Job {
            name: self.name.clone(),
            club: self.club_or_region.club,
            list: self.list.clone(),
            api_key: self.api_key.clone(),
            region: self.club_or_region.region,
            ..Default::default()
        };
        let db = settings.mail.db.connect().await?;
        let job = Job::create(&db, &to_create).await?;
        print_json(&job)
    }
}
