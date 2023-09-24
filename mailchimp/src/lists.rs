use crate::{
    deserialize_null_string, paged_query_impl, paged_response_impl, query_default_impl, Client,
    Stream,
};
use serde::{Deserialize, Serialize};

pub fn all(client: &Client, query: ListsQuery) -> Stream<List> {
    client.fetch_stream::<ListsQuery, ListsResponse>("/3.0/lists", query)
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
