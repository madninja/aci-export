use crate::{DB_DELETE_CHUNK_SIZE, DB_INSERT_CHUNK_SIZE, Error, Result};
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birthday: Option<chrono::NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone_mobile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone_home: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login: Option<chrono::NaiveDate>,
}

pub const FETCH_USER_QUERY: &str = r#"
    SELECT
        id,
        uid,
        email,
        first_name,
        last_name,
        birthday,
        phone_mobile,
        phone_home,
        last_login
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
                    email,
                    uid,
                    first_name,
                    last_name,
                    birthday,
                    last_login
                ) "#,
            )
            .push_values(chunk, |mut b, user| {
                b.push_bind(&user.id)
                    .push_bind(&user.email)
                    .push_bind(user.uid)
                    .push_bind(&user.first_name)
                    .push_bind(&user.last_name)
                    .push_bind(user.birthday)
                    .push_bind(user.last_login);
            })
            .push(
                r#"ON CONFLICT(id) DO UPDATE SET
                email = excluded.email,
                uid = excluded.uid,
                first_name = excluded.first_name,
                last_name = excluded.last_name,
                birthday = excluded.birthday,
                last_login = excluded.last_login
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
    if users.is_empty() {
        return Ok(0);
    }
    let ids: Vec<&String> = users.iter().map(|user| &user.id).collect();
    let mut tx = pool.begin().await?;
    let mut total_affected = 0;
    for chunk in ids.chunks(DB_DELETE_CHUNK_SIZE) {
        let mut builder = sqlx::QueryBuilder::new(r#" DELETE FROM users WHERE id NOT IN ("#);
        let mut seperated = builder.separated(", ");
        for id in chunk {
            seperated.push_bind(id);
        }
        seperated.push_unseparated(") ");
        let result = builder.build().execute(&mut *tx).await?;
        total_affected += result.rows_affected();
    }
    tx.commit().await?;
    Ok(total_affected)
}
