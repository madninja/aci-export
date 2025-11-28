use crate::{DB_DELETE_CHUNK_SIZE, DB_INSERT_CHUNK_SIZE, Error, Result, user};
use futures::{StreamExt, TryStreamExt, stream};
use sqlx::{PgPool, Postgres};

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

pub async fn by_number(pool: &PgPool, number: &str) -> Result<Option<Brn>> {
    let user = fetch_brn_query()
        .push("WHERE number = ")
        .push_bind(number)
        .build_query_as::<Brn>()
        .fetch_optional(pool)
        .await?;

    Ok(user)
}

pub async fn by_email(pool: &PgPool, email: &str) -> Result<Vec<Brn>> {
    let brns = fetch_brn_query()
        .push("WHERE user_id = ")
        .push_bind(user::id_for_email(email))
        .build_query_as::<Brn>()
        .fetch_all(pool)
        .await?;

    Ok(brns)
}

pub async fn upsert_many(pool: &PgPool, brns: &[Brn]) -> Result<u64> {
    if brns.is_empty() {
        return Ok(0);
    }
    let affected: Vec<u64> = stream::iter(brns)
        .chunks(DB_INSERT_CHUNK_SIZE)
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
            .execute(pool)
            .await?;
            Ok::<u64, Error>(result.rows_affected())
        })
        .try_collect()
        .await?;
    Ok(affected.iter().sum())
}

pub async fn retain(pool: &PgPool, users: &[Brn]) -> Result<u64> {
    if users.is_empty() {
        return Ok(0);
    }
    let numbers: Vec<&String> = users.iter().map(|brn| &brn.number).collect();
    let mut tx = pool.begin().await?;
    let mut total_affected = 0;
    for chunk in numbers.chunks(DB_DELETE_CHUNK_SIZE) {
        let mut builder = sqlx::QueryBuilder::new(r#" DELETE FROM brns WHERE number NOT IN ("#);
        let mut seperated = builder.separated(", ");
        for number in chunk {
            seperated.push_bind(number);
        }
        seperated.push_unseparated(") ");
        let result = builder.build().execute(&mut *tx).await?;
        total_affected += result.rows_affected();
    }
    tx.commit().await?;
    Ok(total_affected)
}
