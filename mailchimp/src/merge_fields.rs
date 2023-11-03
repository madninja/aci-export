use crate::{
    deserialize_null_string, error::Error, paged_query_impl, paged_response_impl,
    query_default_impl, Client, Result, Stream, NO_QUERY,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Serialize, Deserialize, PartialEq, Eq, Debug, Clone)]
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

#[derive(PartialEq, Serialize, Deserialize, Eq, Debug, Clone, Default)]
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

impl std::fmt::Display for MergeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = serde_json::to_string(self).map_err(|_| std::fmt::Error)?;
        f.write_str(&str)
    }
}

impl std::str::FromStr for MergeType {
    type Err = Error;
    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        serde_json::from_str(&format!("\"{s}\""))
            .map_err(|_| Error::InvalidMergeType(s.to_string()))
    }
}

#[derive(Debug, Default, Clone)]
pub struct MergeFields(HashMap<String, MergeField>);

impl<'de> Deserialize<'de> for MergeFields {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        HashMap::<String, MergeField>::deserialize(deserializer).map(Self)
    }
}

impl Serialize for MergeFields {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.0.serialize(serializer)
    }
}
impl std::ops::Deref for MergeFields {
    type Target = HashMap<String, MergeField>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for MergeFields {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl FromIterator<MergeField> for MergeFields {
    fn from_iter<T: IntoIterator<Item = MergeField>>(iter: T) -> Self {
        Self(
            iter.into_iter()
                .map(|field| (field.tag.clone(), field))
                .collect(),
        )
    }
}

impl Extend<MergeField> for MergeFields {
    fn extend<T: IntoIterator<Item = MergeField>>(&mut self, iter: T) {
        let iter = iter.into_iter().map(|field| (field.tag.clone(), field));
        self.0.extend(iter)
    }
}

impl IntoIterator for MergeFields {
    type Item = (String, MergeField);
    type IntoIter = std::collections::hash_map::IntoIter<String, MergeField>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl From<HashMap<String, MergeField>> for MergeFields {
    fn from(value: HashMap<String, MergeField>) -> Self {
        Self(value)
    }
}

pub type MergeFieldValue = (String, serde_json::Value);

impl MergeFields {
    pub fn to_value<F>(&self, tag: &str, value: F) -> Result<Option<MergeFieldValue>>
    where
        F: ToMergeFieldValue + std::fmt::Debug,
    {
        match self.get(tag) {
            Some(field) => value.to_merge_field_value(field),
            None => Ok(None),
        }
    }
}

pub trait ToMergeFieldValue {
    fn to_merge_field_value(self, field: &MergeField) -> Result<Option<MergeFieldValue>>
    where
        Self: Sized;
}

impl ToMergeFieldValue for chrono::NaiveDate {
    fn to_merge_field_value(self, field: &MergeField) -> Result<Option<MergeFieldValue>> {
        match field.r#type {
            MergeType::Date => Ok(Some((
                field.tag.clone(),
                self.format("%Y-%m-%d").to_string().into(),
            ))),
            MergeType::Birthday => Ok(Some((
                field.tag.clone(),
                self.format("%m/%d").to_string().into(),
            ))),
            _ => Err(Error::InvalidMergeType(field.r#type.to_string())),
        }
    }
}

impl ToMergeFieldValue for u64 {
    fn to_merge_field_value(self, field: &MergeField) -> Result<Option<MergeFieldValue>> {
        match field.r#type {
            MergeType::Number => Ok(Some((
                field.tag.clone(),
                serde_json::to_value(self).unwrap(),
            ))),
            MergeType::Text => Ok(Some((
                field.tag.clone(),
                serde_json::to_value(self.to_string()).unwrap(),
            ))),
            _ => Err(Error::InvalidMergeType(field.r#type.to_string())),
        }
    }
}

impl ToMergeFieldValue for i64 {
    fn to_merge_field_value(self, field: &MergeField) -> Result<Option<MergeFieldValue>> {
        match field.r#type {
            MergeType::Number => Ok(Some((
                field.tag.clone(),
                serde_json::to_value(self).unwrap(),
            ))),
            MergeType::Text => Ok(Some((
                field.tag.clone(),
                serde_json::to_value(self.to_string()).unwrap(),
            ))),
            _ => Err(Error::InvalidMergeType(field.r#type.to_string())),
        }
    }
}

impl ToMergeFieldValue for &str {
    fn to_merge_field_value(self, field: &MergeField) -> Result<Option<MergeFieldValue>> {
        match field.r#type {
            MergeType::Text => Ok(Some((field.tag.clone(), self.to_string().into()))),
            _ => Err(Error::InvalidMergeType(field.r#type.to_string())),
        }
    }
}

impl ToMergeFieldValue for &String {
    fn to_merge_field_value(self, field: &MergeField) -> Result<Option<MergeFieldValue>> {
        self.as_str().to_merge_field_value(field)
    }
}

impl<T> ToMergeFieldValue for Option<T>
where
    T: ToMergeFieldValue,
{
    fn to_merge_field_value(self, field: &MergeField) -> Result<Option<MergeFieldValue>> {
        match self {
            Some(value) => value.to_merge_field_value(field),
            None => Ok(None),
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
