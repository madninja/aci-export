use crate::{
    deserialize_null_string, error::Error, paged_query_impl, paged_response_impl,
    query_default_impl, Client, Result, Stream, NO_QUERY,
};
use serde::{Deserialize, Serialize};

pub fn all(client: &Client, list_id: &str, query: MergeFieldsQuery) -> Stream<MergeField> {
    client.fetch_stream::<MergeFieldsQuery, MergeFieldsResponse>(
        &format!("/3.0/lists/{list_id}/merge-fields"),
        query,
    )
}

pub async fn get(client: &Client, list_id: &str, merge_id: u32) -> Result<MergeField> {
    client
        .fetch(
            &format!("/3.0/lists/{list_id}/merge-fields/{merge_id}"),
            NO_QUERY,
        )
        .await
}

pub async fn create(client: &Client, list_id: &str, field: MergeField) -> Result<MergeField> {
    client
        .post(&format!("/3.0/lists/{list_id}/merge-fields"), &field)
        .await
}

pub async fn delete(client: &Client, list_id: &str, merge_id: &str) -> Result<()> {
    client
        .delete(&format!("/3.0/lists/{list_id}/merge-fields/{merge_id}"))
        .await
}

pub async fn update(
    client: &Client,
    list_id: &str,
    merge_id: &str,
    field: MergeField,
) -> Result<MergeField> {
    client
        .patch(
            &format!("/3.0/lists/{list_id}/merge-fields/{merge_id}",),
            &field,
        )
        .await
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct MergeField {
    #[serde(
        default,
        skip_serializing_if = "crate::is_default",
        deserialize_with = "crate::deserialize_null_i32::deserialize"
    )]
    pub merge_id: i32,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub tag: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub name: String,

    #[serde(default)]
    pub r#type: MergeType,
}

impl Default for MergeField {
    fn default() -> Self {
        Self {
            merge_id: 0,
            tag: "".to_string(),
            name: "".to_string(),
            r#type: MergeType::default(),
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum MergeType {
    #[default]
    Text,
    Number,
    Address,
    Phone,
    Date,
    Url,
    ImageUrl,
    Radio,
    Dropdown,
    Birthday,
    Zip,
}

impl std::str::FromStr for MergeType {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self> {
        match s {
            "text" => Ok(Self::Text),
            "number" => Ok(Self::Number),
            "address" => Ok(Self::Address),
            "phone" => Ok(Self::Phone),
            "date" => Ok(Self::Date),
            "url" => Ok(Self::Url),
            "imageurl" => Ok(Self::ImageUrl),
            "radio" => Ok(Self::Radio),
            "dropdown" => Ok(Self::Dropdown),
            "birthday" => Ok(Self::Birthday),
            "zip" => Ok(Self::Zip),
            _ => Err(Error::InvalidMergeType(s.to_string())),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MergeFieldsQuery {
    pub fields: String,
    pub count: u32,
    pub offset: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MergeFieldsResponse {
    pub merge_fields: Vec<MergeField>,
}

query_default_impl!(MergeFieldsQuery);
paged_query_impl!(
    MergeFieldsQuery,
    &[
        "merge_fields.merge_id",
        "merge_fields.tag",
        "merge_fields.name",
        "merge_fields.type"
    ]
);
paged_response_impl!(MergeFieldsResponse, merge_fields, MergeField);
