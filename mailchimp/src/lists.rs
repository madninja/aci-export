use crate::{
    deserialize_null_string, Client, PagedQuery, PagedResponse, Stream, DEFAULT_QUERY_COUNT,
};
use serde::{Deserialize, Serialize};

pub fn all(client: &Client) -> Stream<List> {
    let query = ListsQuery::default();
    client.fetch_stream::<ListsQuery, ListsResponse>("/3.0/lists", query)
}

pub mod members {
    use super::*;

    pub fn all(client: &Client, list_id: &str) -> Stream<Member> {
        let query = MembersQuery::default();
        client.fetch_stream::<MembersQuery, MembersResponse>(
            &format!("/3.0/lists/{list_id}/members"),
            query,
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListsQuery {
    pub fields: String,
    pub count: u32,
    pub offset: u32,
}

impl Default for ListsQuery {
    fn default() -> Self {
        Self {
            fields: Self::default_fields_string(),
            count: DEFAULT_QUERY_COUNT,
            offset: 0,
        }
    }
}

impl PagedQuery for ListsQuery {
    fn default_fields() -> &'static [&'static str] {
        &["lists.id", "lists.name"]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListsResponse {
    pub lists: Vec<List>,
}

impl PagedResponse for ListsResponse {
    type Item = List;

    fn pop(&mut self) -> Option<List> {
        self.lists.pop()
    }
    fn len(&self) -> usize {
        self.lists.len()
    }
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
pub struct MembersQuery {
    pub fields: String,
    pub count: u32,
    pub offset: u32,
}

impl Default for MembersQuery {
    fn default() -> Self {
        Self {
            fields: Self::default_fields_string(),
            count: DEFAULT_QUERY_COUNT,
            offset: 0,
        }
    }
}

impl PagedQuery for MembersQuery {
    fn default_fields() -> &'static [&'static str] {
        &["members.id", "members.email_address", "members.full_name"]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MembersResponse {
    pub members: Vec<Member>,
}

impl PagedResponse for MembersResponse {
    type Item = Member;

    fn pop(&mut self) -> Option<Member> {
        self.members.pop()
    }
    fn len(&self) -> usize {
        self.members.len()
    }
}
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
