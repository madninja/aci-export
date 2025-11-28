use crate::{Context, Result};
use config::{Config, Environment};
use serde::Deserialize;
use sqlx::{MySqlPool, PgPool};

#[derive(Debug, Deserialize, Clone, Default)]
pub struct Settings {
    #[serde(default = "default_log")]
    pub log: String,
    pub ddb: AciDatabaseSettings,
    #[serde(default)]
    pub app: AppSettings,
}

impl Settings {
    /// Settings are loaded from environment variables with prefix ACI__
    pub fn new() -> Result<Self> {
        Ok(Config::builder()
            .add_source(
                Environment::with_prefix("ACI")
                    .separator("_")
                    .prefix_separator("__"),
            )
            .build()
            .and_then(|config| config.try_deserialize())?)
    }
}

fn default_log() -> String {
    "sync_app=info,db=info".to_string()
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct AppSettings {
    #[serde(default)]
    pub db: DatabaseSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseSettings {
    #[serde(default = "default_db_url")]
    pub url: String,
}

impl Default for DatabaseSettings {
    fn default() -> Self {
        Self {
            url: default_db_url(),
        }
    }
}

fn default_db_url() -> String {
    "".to_string()
}

impl DatabaseSettings {
    pub async fn connect(&self) -> Result<PgPool> {
        let pool = PgPool::connect(&self.url)
            .await
            .context(format!("opening database {}", self.url))?;
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
