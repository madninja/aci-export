use crate::{deserialize_null_string, Client, PagedQuery, PagedResult, Stream};
use serde::{Deserialize, Serialize};

pub fn all(client: &Client, fields: &[&str]) -> Stream<List> {
    let query = ListsQuery {
        fields: fields.join(","),
        ..Default::default()
    };
    client.fetch_stream::<ListsQuery, ListsResponse>("/3.0/lists", &query)
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ListsQuery {
    pub fields: String,
    pub count: u32,
    pub offset: u32,
}

impl PagedQuery for ListsQuery {
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

impl PagedResult for ListsResponse {
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
