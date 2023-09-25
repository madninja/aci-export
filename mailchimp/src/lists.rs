use crate::{
    deserialize_null_string, paged_query_impl, paged_response_impl, query_default_impl, Client,
    Result, Stream, NO_QUERY,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub fn all(client: &Client, query: ListsQuery) -> Stream<List> {
    client.fetch_stream::<ListsQuery, ListsResponse>("/3.0/lists", query)
}

pub async fn get(client: &Client, list_id: &str) -> Result<List> {
    client
        .fetch(&format!("/3.0/lists/{list_id}"), NO_QUERY)
        .await
}

pub async fn create(client: &Client, list: &List) -> Result<List> {
    client.post(&format!("/3.0/lists"), list).await
}

pub async fn delete(client: &Client, list_id: &str) -> Result<()> {
    client.delete(&format!("/3.0/lists/{list_id}")).await
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct List {
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub id: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact: Option<ListContact>,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub permission_reminder: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub campaign_defaults: Option<CampaignDefaults>,
    #[serde(default)]
    pub email_type_option: bool,
    #[serde(default)]
    pub use_archive_bar: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date_created: Option<DateTime<Utc>>,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub notify_on_subscribe: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub notify_on_unsubscribe: String,
    #[serde(default)]
    pub double_optin: bool,
    #[serde(default)]
    pub has_welcome: bool,
    #[serde(default)]
    pub marketing_permissions: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListsQuery {
    pub fields: String,
    pub count: u32,
    pub offset: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListsResponse {
    pub lists: Vec<List>,
}

paged_query_impl!(ListsQuery, &["lists.id", "lists.name"]);
query_default_impl!(ListsQuery);
paged_response_impl!(ListsResponse, lists, List);

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ListContact {
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub address1: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub address2: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub city: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub company: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub country: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub phone: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub state: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub zip: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct CampaignDefaults {
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub from_email: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub from_name: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub language: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub subject: String,
}
