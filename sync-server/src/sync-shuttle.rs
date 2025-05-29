use anyhow::{Context, Error};
use futures::TryFutureExt;
use shuttle_runtime::SecretStore;
use sqlx::PgPool;
use sync_server::settings::Settings;
use tracing_subscriber::EnvFilter;

#[shuttle_runtime::main]
async fn main(
    #[shuttle_shared_db::Postgres] db: PgPool,
    #[shuttle_runtime::Secrets] secrets: SecretStore,
) -> Result<SyncServer, shuttle_runtime::Error> {
    for (key, secret) in secrets {
        std::env::set_var(key, secret);
    }
    let settings = sync_server::settings::Settings::new()?;
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&settings.log))
        .init();
    let service = SyncServer { db, settings };
    Ok(service)
}

struct SyncServer {
    db: PgPool,
    settings: Settings,
}

#[shuttle_runtime::async_trait]
impl shuttle_runtime::Service for SyncServer {
    async fn bind(self, addr: std::net::SocketAddr) -> Result<(), shuttle_runtime::Error> {
        tracing::info!("running migrations");
        sqlx::migrate!()
            .run(&self.db)
            .map_err(Error::from)
            .await
            .context("running migrations")?;
        sync_server::server::run(self.settings, self.db, addr).await?;

        Ok(())
    }
}
