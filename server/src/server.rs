use crate::{cron, settings::Settings, Error, Result};
use tokio_graceful_shutdown::{SubsystemBuilder, Toplevel};

pub async fn run(settings: Settings) -> Result {
    Toplevel::new(move |top_level| async move {
        top_level.start(SubsystemBuilder::new("cron", {
            move |handle| cron::subsystem(settings, handle)
        }));
    })
    .catch_signals()
    .handle_shutdown_requests(tokio::time::Duration::from_secs(5))
    .await
    .map_err(Error::from)
}
