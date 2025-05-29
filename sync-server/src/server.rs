use crate::{api, cron, settings::Settings, Error, Result};
use sqlx::PgPool;
use tokio_graceful_shutdown::{SubsystemBuilder, Toplevel};

pub async fn run(settings: Settings, db: PgPool, addr: std::net::SocketAddr) -> Result {
    Toplevel::new(move |top_level| async move {
        let cron_db = db.clone();
        top_level.start(SubsystemBuilder::new("cron", {
            move |handle| cron::subsystem(settings, cron_db, handle)
        }));
        let api_db = db.clone();
        top_level.start(SubsystemBuilder::new("api", {
            move |handle| api::subsystem(addr, api_db, handle)
        }));
    })
    .catch_signals()
    .handle_shutdown_requests(tokio::time::Duration::from_secs(5))
    .await
    .map_err(Error::from)
}
