use crate::{Error, Result};
use anyhow::{anyhow, bail, Context};
use config::{Config, Environment, File};
use serde::Deserialize;
use sqlx::MySqlPool;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub mailchimp: MailchimpSetting,
}

impl Settings {
    /// Settings are loaded from the file in the given path.
    pub fn new(path: &Path, env_prefix: &str) -> Result<Self> {
        Ok(Config::builder()
            // Source settings file
            .add_source(File::with_name(path.to_str().expect("file name")).required(false))
            .add_source(Environment::with_prefix("ACI").separator("__"))
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
        Ok(pool)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct MailchimpSetting {
    pub api_key: String,
    pub fields: String,
    pub club: Option<u64>,
    pub region: Option<u64>,
    pub list: Option<String>,
}

impl MailchimpSetting {
    pub fn client(&self) -> Result<mailchimp::Client> {
        Ok(mailchimp::client::from_api_key(&self.api_key)?)
    }

    pub fn fields(&self) -> Result<mailchimp::merge_fields::MergeFields> {
        read_merge_fields(&self.fields)
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
    impl TryFrom<MergeFieldsConfig> for mailchimp::merge_fields::MergeFields {
        type Error = Error;
        fn try_from(config: MergeFieldsConfig) -> Result<Self> {
            for field in config.merge_fields.iter() {
                if field.tag.len() > 10 {
                    bail!("Merge field tag too long: {}", field.tag);
                }
            }
            Ok(config.merge_fields.into_iter().collect())
        }
    }

    read_toml::<MergeFieldsConfig>(path).and_then(TryInto::try_into)
}
