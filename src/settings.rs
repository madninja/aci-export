use crate::Result;
use anyhow::Context;
use config::{Config, Environment, File};
use serde::Deserialize;
use sqlx::MySqlPool;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
}

impl Settings {
    /// Settings are loaded from the file in the given path.
    pub fn new(path: &Path) -> Result<Self> {
        Ok(Config::builder()
            // Source settings file
            .add_source(File::with_name(path.to_str().expect("file name")).required(false))
            .add_source(Environment::with_prefix("ACI").separator("__"))
            .build()
            .and_then(|config| config.try_deserialize())?)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseSettings {
    pub url: String,
}

impl DatabaseSettings {
    pub async fn connect(&self) -> Result<MySqlPool> {
        let pool = MySqlPool::connect(&self.url)
            .await
            .context("opening database")?;
        Ok(pool)
    }
}
