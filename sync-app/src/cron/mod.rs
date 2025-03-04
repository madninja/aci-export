use crate::{settings::Settings, Result};
use tokio_cron_scheduler::JobScheduler;
use tokio_graceful_shutdown::SubsystemHandle;

pub mod sync_db;

pub async fn subsystem(settings: Settings, handle: SubsystemHandle) -> Result<()> {
    let mut scheduler = JobScheduler::new().await?;
    tracing::info!("started scheduler");
    sync_db::schedule(settings, &mut scheduler).await?;
    scheduler.start().await?;
    handle.on_shutdown_requested().await;

    tracing::info!("stopped scheduler");
    scheduler.shutdown().await?;
    Ok(())
}
