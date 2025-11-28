use crate::{Result, cmd::print_json, mailchimp::Job, settings::Settings};

/// List the configured sync jobs
#[derive(Debug, clap::Args)]
pub struct Cmd {}

impl Cmd {
    pub async fn run(&self, settings: Settings) -> Result {
        let db = settings.mail.db.connect().await?;
        let jobs = Job::all(&db).await?;
        print_json(&jobs)
    }
}
