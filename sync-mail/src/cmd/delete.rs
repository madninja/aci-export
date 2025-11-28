use crate::{Result, cmd::print_json, mailchimp::Job, settings::Settings};
use serde_json::json;

/// Delete a sync job
///
/// Without the confirm flag this just lists the job that would be deleted
#[derive(Debug, clap::Args)]
pub struct Cmd {
    id: i64,
    #[arg(long)]
    confirm: bool,
}

impl Cmd {
    pub async fn run(&self, settings: Settings) -> Result {
        let db = settings.mail.db.connect().await?;
        let job = Job::get(&db, self.id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("no such job"))?;
        if self.confirm {
            Job::delete(&db, self.id).await?;
            print_json(&json!({ "deleted": "ok" }))
        } else {
            print_json(&job)
        }
    }
}
