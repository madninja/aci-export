use crate::{Error, Result};
use futures::{stream, StreamExt, TryStreamExt};
use sqlx::{postgres::PgExecutor, Postgres};

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct User {
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

impl From<ddb::users::User> for User {
    fn from(value: ddb::users::User) -> Self {
        Self {
            uid: value.uid as i64,
            email: value.email,
            first_name: value.first_name,
            last_name: value.last_name,
            birthday: value.birthday,
            phone_mobile: None,
            phone_home: None,
            last_login: value.last_login,
        }
    }
}

pub const FETCH_USER_QUERY: &str = r#"
    SELECT
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

pub async fn by_uid<'c, E>(exec: E, uid: i64) -> Result<Option<User>>
where
    E: PgExecutor<'c>,
{
    let user = fetch_user_query()
        .push("WHERE uid = ")
        .push_bind(uid)
        .build_query_as::<User>()
        .fetch_optional(exec)
        .await?;

    Ok(user)
}

pub async fn by_email<'c, E>(exec: E, email: &str) -> Result<Option<User>>
where
    E: PgExecutor<'c>,
{
    let user = fetch_user_query()
        .push("WHERE email = ")
        .push_bind(email)
        .build_query_as::<User>()
        .fetch_optional(exec)
        .await?;

    Ok(user)
}
pub async fn upsert_many<'c, E>(exec: E, users: &[User]) -> Result<u64>
where
    E: PgExecutor<'c> + Copy,
{
    if users.is_empty() {
        return Ok(0);
    }
    let affected: Vec<u64> = stream::iter(users)
        .chunks(5000)
        .map(Ok)
        .and_then(|chunk| async move {
            let result = sqlx::QueryBuilder::new(
                r#"INSERT INTO users (
            email,
            uid,
            first_name,
            last_name,
            birthday,
            last_login
        ) "#,
            )
            .push_values(chunk, |mut b, user| {
                b.push_bind(&user.email)
                    .push_bind(&user.uid)
                    .push_bind(&user.first_name)
                    .push_bind(&user.last_name)
                    .push_bind(&user.birthday)
                    .push_bind(&user.last_login);
            })
            .push(
                r#"ON CONFLICT(email) DO UPDATE SET
                uid = excluded.uid,
                first_name = excluded.first_name,
                last_name = excluded.last_name,
                birthday = excluded.birthday,
                last_login = excluded.last_login
            "#,
            )
            .build()
            .execute(exec)
            .await?;
            Ok::<u64, Error>(result.rows_affected())
        })
        .try_collect()
        .await?;
    Ok(affected.iter().sum())
}

pub async fn retain<'c, E>(exec: E, users: &[User]) -> Result<u64>
where
    E: PgExecutor<'c>,
{
    if users.is_empty() {
        return Ok(0);
    }
    let mut builder = sqlx::QueryBuilder::new(r#" DELETE FROM users WHERE email NOT IN ("#);
    let mut seperated = builder.separated(", ");
    for user in users {
        seperated.push_bind(&user.email);
    }
    seperated.push_unseparated(") ");
    let result = builder.build().execute(exec).await?;
    Ok(result.rows_affected())
}
