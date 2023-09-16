use crate::{Error, Result};
use serde::{Deserialize, Serialize};

pub mod client {
    use super::*;

    pub fn from_api_key(api_key: &str) -> Result<mailchimp::Client> {
        let auth = mailchimp::AuthMode::new_basic_auth(api_key)?;
        Ok(mailchimp::Client::new(auth))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct List {
    pub id: String,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stats: Option<mailchimp_api::types::Stats>,
}

impl From<mailchimp_api::types::Lists> for List {
    fn from(value: mailchimp_api::types::Lists) -> Self {
        Self {
            id: value.id,
            name: value.name,
            stats: value.stats,
        }
    }
}

pub async fn lists(client: &mailchimp::Client) -> Result<Vec<List>> {
    let client_lists = client.lists();

    const PAGE_SIZE: i64 = 100;
    let mut offset: i64 = 0;
    let mut result: Vec<List> = vec![];

    loop {
        let page: Vec<List> = client_lists
            .get(
                &["lists.name,lists.id".to_string()],
                &[],
                PAGE_SIZE,
                offset,
                "",
                "",
                "",
                "",
                "",
                Default::default(),
                Default::default(),
                false,
                false,
            )
            .await?
            .lists
            .into_iter()
            .map(List::from)
            .collect();
        if page.is_empty() {
            break;
        }
        offset += PAGE_SIZE;
        result.extend(page);
    }
    Ok(result)
}

pub async fn ping(client: &Client) -> Result<mailchimp_api::types::ApiHealthStatus> {
    client.ping().get().await.map_err(Error::from)
}

pub mod list {
    use super::*;

    pub async fn info(client: &Client, id: &str) -> Result<List> {
        client
            .lists()
            .get_lists(&["id,name,stats".to_string()], &[], id, false)
            .await
            .map_err(Error::from)
            .map(List::from)
    }
}
