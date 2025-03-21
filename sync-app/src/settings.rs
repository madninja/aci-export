use crate::{Context, Result};
use config::{Config, Environment};
use serde::Deserialize;
use sqlx::{MySqlPool, PgPool};

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Settings {
    #[serde(default = "default_log")]
    pub log: String,
    #[serde(default)]
    pub db: DatabaseSettings,
    pub ddb: AciDatabaseSettings,
}
impl Settings {
    /// Settings are loaded from the file in the given path.
    pub fn new() -> Result<Self> {
        Ok(Config::builder()
            // Source settings file
            .add_source(
                Environment::with_prefix("APP")
                    .separator("_")
                    .prefix_separator("__"),
            )
            .build()
            .and_then(|config| config.try_deserialize())?)
    }
}

fn default_log() -> String {
    "sync_app=info".to_string()
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseSettings {
    #[serde(default = "default_db_url")]
    pub url: String,
}

fn default_db_url() -> String {
    "postgresql://postgres:postgres@127.0.0.1:54322/postgres".to_string()
}

impl Default for DatabaseSettings {
    fn default() -> Self {
        Self {
            url: default_db_url(),
        }
    }
}

impl DatabaseSettings {
    pub async fn connect(&self) -> Result<PgPool> {
        let pool = PgPool::connect(&self.url)
            .await
            .context("opening database")?;
        Ok(pool)
    }
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct AciDatabaseSettings {
    #[serde(default = "default_ddb_url")]
    pub url: String,
}

fn default_ddb_url() -> String {
    "".to_string()
}

impl AciDatabaseSettings {
    pub async fn connect(&self) -> Result<MySqlPool> {
        ddb::connect(&self.url).await.context("opening database")
    }
}
