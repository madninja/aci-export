use crate::{DB_INSERT_CHUNK_SIZE, Error, Result, retain_with_keys};
use futures::{StreamExt, TryStreamExt, stream};
use sqlx::{PgPool, Postgres};

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct User {
    pub id: String,
    pub uid: i64,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
}

pub const FETCH_USER_QUERY: &str = r#"
    SELECT
        id,
        uid,
        email,
        first_name,
        last_name
    FROM
        users
"#;

fn fetch_user_query<'builder>() -> sqlx::QueryBuilder<'builder, Postgres> {
    sqlx::QueryBuilder::new(FETCH_USER_QUERY)
}

pub fn id_for_email(email: &str) -> String {
    use base64::prelude::*;
    use sha2::{Digest, Sha256};
    BASE64_URL_SAFE_NO_PAD.encode(Sha256::digest(email.trim().to_lowercase()))
}

pub async fn by_uid(pool: &PgPool, uid: i64) -> Result<Option<User>> {
    let user = fetch_user_query()
        .push("WHERE uid = ")
        .push_bind(uid)
        .build_query_as::<User>()
        .fetch_optional(pool)
        .await?;

    Ok(user)
}

pub async fn by_email(pool: &PgPool, email: &str) -> Result<Option<User>> {
    let user = fetch_user_query()
        .push("WHERE id = ")
        .push_bind(id_for_email(email))
        .build_query_as::<User>()
        .fetch_optional(pool)
        .await?;

    Ok(user)
}

pub async fn upsert_many(pool: &PgPool, users: &[User]) -> Result<u64> {
    if users.is_empty() {
        return Ok(0);
    }
    let affected: Vec<u64> = stream::iter(users)
        .chunks(DB_INSERT_CHUNK_SIZE)
        .map(Ok)
        .and_then(|chunk| async move {
            let result = sqlx::QueryBuilder::new(
                r#"INSERT INTO users (
                    id,
                    uid,
                    email,
                    first_name,
                    last_name
                ) "#,
            )
            .push_values(chunk, |mut b, user| {
                b.push_bind(&user.id)
                    .push_bind(user.uid)
                    .push_bind(&user.email)
                    .push_bind(&user.first_name)
                    .push_bind(&user.last_name);
            })
            .push(
                r#"ON CONFLICT(id) DO UPDATE SET
                uid = excluded.uid,
                email = excluded.email,
                first_name = excluded.first_name,
                last_name = excluded.last_name
            "#,
            )
            .build()
            .execute(pool)
            .await?;
            Ok::<u64, Error>(result.rows_affected())
        })
        .try_collect()
        .await?;
    Ok(affected.iter().sum())
}

pub async fn retain(pool: &PgPool, users: &[User]) -> Result<u64> {
    retain_with_keys(pool, "users", "id", users, |user| user.id.as_str()).await
}
