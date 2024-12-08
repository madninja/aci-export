use crate::{
    batches, deserialize_null_string, paged_query_impl, paged_response_impl, query_default_impl,
    Client, Error, Result, Stream, NO_QUERY,
};
use futures::stream::{Stream as StdStream, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::RwLock;

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

/// Delete any members from the given list id not in a given retained set
///
/// Returns the numebr of deleted members from the given list
pub async fn retain(client: &Client, list_id: &str, keep_keys: &HashSet<String>) -> Result<usize> {
    // Iterate through all mailchimp audience member. Collect all members that are not
    // the upserted set by set subtraction
    let mailchimp_stream = all(client, list_id, Default::default());
    let audience: HashSet<String> = mailchimp_stream
        .map_err(Error::from)
        .try_filter_map(|member| async move {
            Ok((member.status != Some(MemberStatus::Cleaned)).then_some(member.id))
        })
        .try_collect::<Vec<_>>()
        .await?
        .into_iter()
        .collect();

    // don't process deletes for a single member sync
    let to_delete = &audience - keep_keys;
    // Delete all to_delete entries
    futures::stream::iter(to_delete.iter())
        .map(|member_id| Ok::<_, crate::Error>((client.clone(), member_id)))
        .try_for_each_concurrent(10, |(client, member_id)| async move {
            delete(&client, list_id, member_id).await?;
            Ok(())
        })
        .await?;
    Ok(to_delete.len())
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

/// Recommended max batch upsert size.
///
/// The Mailchimp docs state that batches up to 500 can be upserted
/// but in practice that size ends up timing out requests.   
pub const MEMBER_BATCH_UPSERT_MAX: usize = 300;

/// Upsert a given list of members into the given list
///
/// Retursn the list of ids of upserted members
pub async fn upsert_many(
    client: &Client,
    list_id: &str,
    members: impl StdStream<Item = Member>,
) -> Result<HashSet<String>> {
    let upserted = Arc::new(RwLock::new(HashSet::new()));
    // chunk in max sizes and yse batch_upsert to upsert the members in the list
    members
        .chunks(MEMBER_BATCH_UPSERT_MAX)
        .map(Ok::<Vec<_>, Error>)
        .map_ok(|members| (client.clone(), members, upserted.clone()))
        .try_for_each_concurrent(8, |(client, members, processed)| async move {
            let response = batch_upsert(&client, list_id, &members).await?;
            let mut set = processed.write().await;
            response
                .updated_members
                .into_iter()
                .chain(response.new_members)
                .for_each(|entry| {
                    set.insert(entry.id);
                });
            if response.error_count > 0 {
                response.errors.iter().for_each(|err| {
                    tracing::warn!(
                        email = err.email_address,
                        err = err.error,
                        "mailchimp error"
                    );
                })
            }
            Ok(())
        })
        .await?;
    let inner = upserted.read_owned().await;
    Ok(inner.to_owned())
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

    pub async fn update_many(
        client: &Client,
        list_id: &str,
        tag_updates: &[(String, Vec<MemberTagUpdate>)],
    ) -> Result {
        futures::stream::iter(tag_updates)
            .chunks(300)
            .map(Ok::<Vec<_>, Error>)
            .map_ok(|updates| (client.clone(), updates))
            .try_for_each_concurrent(10, |(client, updates)| async move {
                let mut batch = batches::Batch::default();
                for (member_id, updates) in updates {
                    let operation = batch::update(&mut batch, list_id, member_id, updates)?;
                    operation.operation_id = member_id.to_owned();
                }
                batch.run(&client, true).await?;
                Ok(())
            })
            .await
    }

    pub mod batch {
        use super::*;

        pub fn update<'a>(
            batch: &'a mut batches::Batch,
            list_id: &str,
            member_id: &str,
            updates: &[MemberTagUpdate],
        ) -> Result<&'a mut batches::BatchOperation> {
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
