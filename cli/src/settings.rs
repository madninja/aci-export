use crate::Result;
use anyhow::{anyhow, Context};
use config::{Config, Environment, File};
use serde::Deserialize;
use sqlx::{Executor, MySqlPool};
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub mail: MailSetting,
}

impl Settings {
    /// Settings are loaded from the file in the given path.
    pub fn new(path: &Path, env_prefix: &str) -> Result<Self> {
        Ok(Config::builder()
            // Source settings file
            .add_source(File::with_name(path.to_str().expect("file name")).required(false))
            .add_source(Environment::with_prefix("APP").separator("__"))
            .add_source(Environment::with_prefix(env_prefix).separator("__"))
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
        let _ = pool
            .execute(
                r#"
            SET GLOBAL table_definition_cache = 4096;
            SET GLOBAL table_open_cache = 4096;
        "#,
            )
            .await
            .context("preparing database caches")?;
        Ok(pool)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct MailSetting {
    pub api_key: String,
    pub fields: String,
    pub club: Option<u64>,
    pub region: Option<u64>,
    pub list: Option<String>,
}

impl MailSetting {
    pub fn client(&self) -> Result<mailchimp::Client> {
        Ok(mailchimp::client::from_api_key(&self.api_key)?)
    }

    pub fn fields(&self) -> Result<mailchimp::merge_fields::MergeFields> {
        mailchimp::merge_fields::MergeFields::from_config(config::File::with_name(&self.fields))
            .map_err(crate::Error::from)
    }

    pub fn list_override<'a>(&'a self, list: &'a Option<String>) -> Result<&'a str> {
        list.as_deref()
            .or(self.list.as_deref())
            .ok_or_else(|| anyhow!("no list id found"))
    }

    pub fn club_override(&self, uid: Option<u64>) -> Option<u64> {
        uid.or(self.club)
    }

    pub fn region_override(&self, uid: Option<u64>) -> Option<u64> {
        uid.or(self.region)
    }

    pub fn fields_override<'a>(&'a self, fields: &'a Option<String>) -> Result<&'a str> {
        fields
            .as_deref()
            .or(Some(&self.fields))
            .ok_or_else(|| anyhow!("no list id found"))
    }
}
