use crate::{settings::Settings, Result};
use sqlx::PgPool;
use tokio_cron_scheduler::JobScheduler;
use tokio_graceful_shutdown::SubsystemHandle;

pub mod db;
pub mod mailchimp;

pub async fn subsystem(settings: Settings, db: PgPool, handle: SubsystemHandle) -> Result<()> {
    let mut scheduler = JobScheduler::new().await?;
    tracing::info!("started scheduler");
    mailchimp::schedule(db, &settings.ddb, &mut scheduler).await?;
    db::schedule(&settings.app, &settings.ddb, &mut scheduler).await?;
    scheduler.start().await?;
    handle.on_shutdown_requested().await;

    tracing::info!("stopped scheduler");
    scheduler.shutdown().await?;
    Ok(())
}
