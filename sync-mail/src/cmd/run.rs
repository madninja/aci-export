use crate::{cmd::print_json, mailchimp::Job, settings::Settings, Result};

/// Sync the given club (or all) mailing list from the membership database
#[derive(Debug, clap::Args)]
pub struct Cmd {
    /// The id of the mailing list to sync
    id: Option<u64>,
}

impl Cmd {
    pub async fn run(&self, settings: Settings) -> Result {
        let db = settings.mail.db.connect().await?;
        let jobs = if let Some(id) = self.id {
            let job = Job::get(&db, id as i64)
                .await?
                .ok_or_else(|| anyhow::anyhow!("sync job not found"))?;
            vec![job]
        } else {
            Job::all(&db).await?
        };

        let map = Job::sync_many(jobs, settings.ddb).await?;
        print_json(&map)
    }
}
