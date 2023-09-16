use crate::{
    deserialize_null_string, Client, PagedQuery, PagedResponse, Stream, DEFAULT_QUERY_COUNT,
};
use serde::{Deserialize, Serialize};

pub fn all(client: &Client, query: ListsQuery) -> Stream<List> {
    client.fetch_stream::<ListsQuery, ListsResponse>("/3.0/lists", query)
}

pub mod members {
    use super::*;

    pub fn all(client: &Client, list_id: &str, query: MembersQuery) -> Stream<Member> {
        client.fetch_stream::<MembersQuery, MembersResponse>(
            &format!("/3.0/lists/{list_id}/members"),
            query,
        )
    }
}

pub mod merge_fields {
    use super::*;

    pub fn all(client: &Client, list_id: &str, query: MergeFieldsQuery) -> Stream<MergeField> {
        client.fetch_stream::<MergeFieldsQuery, MergeFieldsResponse>(
            &format!("/3.0/lists/{list_id}/merge-fields"),
            query,
        )
    }
}

macro_rules! paged_query_impl {
    ($query_type:ident, $default_fields:expr) => {
        impl PagedQuery for $query_type {
            fn default_fields() -> &'static [&'static str] {
                $default_fields
            }

            fn set_count(&mut self, count: u32) {
                self.count = count;
            }

            fn offset(&self) -> u32 {
                self.offset
            }

            fn set_offset(&mut self, offset: u32) {
                self.offset = offset;
            }
        }
    };
}

macro_rules! paged_response_impl {
    ($response_type:ident, $item_field:ident, $item_type:ident) => {
        impl PagedResponse for $response_type {
            type Item = $item_type;

            fn pop(&mut self) -> Option<$item_type> {
                self.$item_field.pop()
            }
            fn len(&self) -> usize {
                self.$item_field.len()
            }
        }
    };
}

macro_rules! query_default_impl {
    ($query_type:ident) => {
        impl Default for $query_type {
            fn default() -> Self {
                Self {
                    fields: Self::default_fields_string(),
                    count: DEFAULT_QUERY_COUNT,
                    offset: 0,
                }
            }
        }
    };
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
pub struct Member {
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
    pub email_address: String,
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub full_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MembersQuery {
    pub fields: String,
    pub count: u32,
    pub offset: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MembersResponse {
    pub members: Vec<Member>,
}

query_default_impl!(MembersQuery);
paged_query_impl!(
    MembersQuery,
    &["members.id", "members.email_address", "members.full_name"]
);
paged_response_impl!(MembersResponse, members, Member);

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct MergeField {
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub merge_id: String,
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
