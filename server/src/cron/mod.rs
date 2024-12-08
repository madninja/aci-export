use crate::{settings::Settings, Result};
use tokio_cron_scheduler::JobScheduler;
use tokio_graceful_shutdown::SubsystemHandle;

pub mod mailchimp;

pub async fn subsystem(settings: Settings, handle: SubsystemHandle) -> Result<()> {
    let db = settings.db.connect().await?;
    let mut scheduler = JobScheduler::new().await?;
    tracing::info!("started scheduler");
    mailchimp::Job::schedule(&db, &settings.ddb, &mut scheduler).await?;
    scheduler.start().await?;
    handle.on_shutdown_requested().await;

    tracing::info!("stopped scheduler");
    scheduler.shutdown().await?;
    Ok(())
}
