use crate::{user, Error, Result};
use futures::{stream, StreamExt, TryStreamExt};
use sqlx::{postgres::PgExecutor, Postgres};

#[derive(Debug, sqlx::FromRow, serde::Serialize, Clone)]
pub struct Brn {
    pub user_id: String,
    pub number: String,
}

pub const FETCH_BRN_QUERY: &str = r#"
    SELECT
        user_id,
        number
    FROM
        brns
"#;

fn fetch_brn_query<'builder>() -> sqlx::QueryBuilder<'builder, Postgres> {
    sqlx::QueryBuilder::new(FETCH_BRN_QUERY)
}

pub async fn by_number<'c, E>(exec: E, number: &str) -> Result<Option<Brn>>
where
    E: PgExecutor<'c>,
{
    let user = fetch_brn_query()
        .push("WHERE number = ")
        .push_bind(number)
        .build_query_as::<Brn>()
        .fetch_optional(exec)
        .await?;

    Ok(user)
}

pub async fn by_email<'c, E>(exec: E, email: &str) -> Result<Vec<Brn>>
where
    E: PgExecutor<'c>,
{
    let brns = fetch_brn_query()
        .push("WHERE user_id = ")
        .push_bind(user::id_for_email(email))
        .build_query_as::<Brn>()
        .fetch_all(exec)
        .await?;

    Ok(brns)
}
pub async fn upsert_many<'c, E>(exec: E, brns: &[Brn]) -> Result<u64>
where
    E: PgExecutor<'c> + Copy,
{
    if brns.is_empty() {
        return Ok(0);
    }
    let affected: Vec<u64> = stream::iter(brns)
        .chunks(1000)
        .map(Ok)
        .and_then(|chunk| async move {
            let result = sqlx::QueryBuilder::new(
                r#"INSERT INTO brns (
                    user_id,
                    number
                ) "#,
            )
            .push_values(chunk, |mut b, brn| {
                b.push_bind(&brn.user_id).push_bind(&brn.number);
            })
            .push(
                r#"ON CONFLICT(number) DO UPDATE SET
                user_id = excluded.user_id
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

pub async fn retain<'c, E>(exec: E, users: &[Brn]) -> Result<u64>
where
    E: PgExecutor<'c>,
{
    if users.is_empty() {
        return Ok(0);
    }
    let mut builder = sqlx::QueryBuilder::new(r#" DELETE FROM brns WHERE number NOT IN ("#);
    let mut seperated = builder.separated(", ");
    for brn in users {
        seperated.push_bind(&brn.number);
    }
    seperated.push_unseparated(") ");
    let result = builder.build().execute(exec).await?;
    Ok(result.rows_affected())
}
