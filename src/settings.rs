use crate::Result;
use anyhow::{anyhow, Context};
use config::{Config, Environment, File};
use serde::Deserialize;
use sqlx::MySqlPool;
use std::{collections::HashMap, path::Path};

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub mailchimp: HashMap<String, MailchimpSetting>,
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

    pub fn profile(&self, profile: &str) -> Result<&MailchimpSetting> {
        self.mailchimp
            .get(profile)
            .ok_or_else(|| anyhow::anyhow!("no mailchimp profile named {profile}"))
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

#[derive(Debug, Deserialize, Clone)]
pub struct MailchimpSetting {
    pub api_key: String,
    pub config: String,
    pub fields: String,
    pub club: Option<u64>,
    pub list: Option<String>,
}

impl MailchimpSetting {
    pub fn client(&self) -> Result<mailchimp::Client> {
        Ok(mailchimp::client::from_api_key(&self.api_key)?)
    }

    pub fn config(&self) -> Result<mailchimp::lists::List> {
        read_toml(&self.config)
    }

    pub fn fields(&self) -> Result<mailchimp::merge_fields::MergeFields> {
        read_merge_fields(&self.fields)
    }

    pub fn list_override<'a>(&'a self, list: &'a Option<String>) -> Result<&'a str> {
        list.as_deref()
            .or(self.list.as_deref())
            .ok_or_else(|| anyhow!("no list id found"))
    }
}

pub fn read_toml<'de, T: serde::Deserialize<'de>>(path: &str) -> Result<T> {
    let config = config::Config::builder()
        .add_source(config::File::with_name(path))
        .build()
        .and_then(|config| config.try_deserialize())?;
    Ok(config)
}

pub fn read_merge_fields(path: &str) -> Result<mailchimp::merge_fields::MergeFields> {
    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    struct MergeFieldsConfig {
        merge_fields: Vec<mailchimp::merge_fields::MergeField>,
    }
    impl From<MergeFieldsConfig> for mailchimp::merge_fields::MergeFields {
        fn from(config: MergeFieldsConfig) -> Self {
            config.merge_fields.into_iter().collect()
        }
    }

    read_toml::<MergeFieldsConfig>(path).map(Into::into)
}
