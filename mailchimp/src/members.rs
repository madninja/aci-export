use crate::{
    deserialize_null_string, paged_query_impl, paged_response_impl, query_default_impl, Client,
    Result, Stream, NO_QUERY,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub fn all(client: &Client, list_id: &str, query: MembersQuery) -> Stream<Member> {
    client.fetch_stream::<MembersQuery, MembersResponse>(
        &format!("/3.0/lists/{list_id}/members"),
        query,
    )
}

pub async fn get(client: &Client, list_id: &str, member_id: &str) -> Result<Member> {
    client
        .fetch(
            &format!("/3.0/lists/{list_id}/members/{member_id}"),
            NO_QUERY,
        )
        .await
}

pub async fn for_email(client: &Client, list_id: &str, email: &str) -> Result<Member> {
    let member_id = format!("{:x}", md5::compute(email.to_lowercase()));
    get(client, list_id, &member_id).await
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum MemberStatus {
    #[default]
    Subscribed,
    Unsubscribed,
    Cleaned,
    Pending,
    Transactional,
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
    pub status: MemberStatus,
    #[serde(default, skip_serializing_if = "crate::is_default")]
    pub merge_fields: HashMap<String, String>,
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
    &[
        "members.id",
        "members.email_address",
        "members.full_name",
        "members.status"
    ]
);
paged_response_impl!(MembersResponse, members, Member);
