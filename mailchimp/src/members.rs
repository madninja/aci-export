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

pub async fn delete(client: &Client, list_id: &str, member_id: &str) -> Result<()> {
    client
        .delete(&format!("/3.0/lists/{list_id}/members/{member_id}"))
        .await
}

pub async fn for_email(client: &Client, list_id: &str, email: &str) -> Result<Member> {
    for_id(client, list_id, &member_id(email)).await
}

pub async fn for_id(client: &Client, list_id: &str, member_id: &str) -> Result<Member> {
    get(client, list_id, member_id).await
}

pub fn member_id(email: &str) -> String {
    format!("{:x}", md5::compute(email.to_lowercase()))
}

pub fn is_valid_email(email: &str) -> bool {
    let email = email.to_lowercase();
    !(email.is_empty() || email.ends_with("noemail.com") || email.ends_with("example.com"))
}

pub async fn upsert(
    client: &Client,
    list_id: &str,
    member_id: &str,
    member: &Member,
) -> Result<Member> {
    client
        .put(
            &format!("/3.0/lists/{list_id}/members/{member_id}",),
            member,
        )
        .await
}

#[derive(Default, Debug, Deserialize)]
pub struct MemberBatchUpsertResponse {
    pub updated_members: Vec<Member>,
    pub new_members: Vec<Member>,
    pub total_created: u16,
    pub total_updated: u16,
    pub error_count: u16,
    pub errors: Vec<MemberBatchUpsertError>,
}

#[derive(Default, Debug, Deserialize)]
pub struct MemberBatchUpsertError {
    pub email_address: String,
    pub error: String,
    pub error_code: String,
    pub field: Option<String>,
    pub field_message: Option<String>,
}

/// Recommended max batch upsert size.
///
/// The Mailchimp docs state that batches up to 500 can be upserted
/// but in practice that size ends up timing out requests.   
pub const MEMBER_BATCH_UPSERT_MAX: usize = 300;

pub async fn batch_upsert(
    client: &Client,
    list_id: &str,
    members: &[Member],
) -> Result<MemberBatchUpsertResponse> {
    #[derive(Serialize, Default)]
    struct MemberBatchUpsertRequest<'a> {
        members: &'a [Member],
        update_existing: bool,
    }
    let batch_request = MemberBatchUpsertRequest {
        members,
        update_existing: true,
    };
    client
        .post(&format!("/3.0/lists/{list_id}/",), &batch_request)
        .await
}

pub mod tags {
    use super::*;

    pub async fn for_id(client: &Client, list_id: &str, member_id: &str) -> Result<Vec<MemberTag>> {
        client
            .fetch(
                &format!("/3.0/lists/{list_id}/members/{member_id}/tags"),
                NO_QUERY,
            )
            .await
    }

    #[derive(Debug, Serialize)]
    struct TagsUpdateRequestBody<'a> {
        tags: &'a [MemberTagUpdate],
    }

    fn tags_update_path(prefix: &str, list_id: &str, member_id: &str) -> String {
        format!("{prefix}/lists/{list_id}/members/{member_id}/tags")
    }

    pub async fn update(
        client: &Client,
        list_id: &str,
        member_id: &str,
        updates: &[MemberTagUpdate],
    ) -> Result {
        let body = TagsUpdateRequestBody { tags: updates };
        client
            .post(&tags_update_path("/3.0", list_id, member_id), &body)
            .await
    }

    pub mod batch {
        use super::*;
        use crate::batches::{Batch, BatchOperation};

        pub fn update<'a>(
            batch: &'a mut Batch,
            list_id: &str,
            member_id: &str,
            updates: &[MemberTagUpdate],
        ) -> Result<&'a mut BatchOperation> {
            let body = TagsUpdateRequestBody { tags: updates };
            batch.post(&tags_update_path("", list_id, member_id), &body)
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum MemberStatus {
    Subscribed,
    Unsubscribed,
    Cleaned,
    Pending,
    Transactional,
    Archived,
    #[default]
    Noop,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_if_new: Option<MemberStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<MemberStatus>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub merge_fields: Option<HashMap<String, serde_json::Value>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags_count: Option<u16>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<MemberTag>,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct MemberTag {
    name: String,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct MemberTagUpdate {
    pub name: String,
    pub status: MemberTagStatus,
}

#[derive(Serialize, Deserialize, PartialEq, Debug, Clone, Default)]
#[serde(rename_all = "lowercase")]
pub enum MemberTagStatus {
    #[default]
    Active,
    Inactive,
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
    &["members.id", "members.email_address", "members.full_name",]
);
paged_response_impl!(MembersResponse, members, Member);
