use crate::{DB_INSERT_CHUNK_SIZE, Error, Result, retain_with_keys, user};
use futures::{StreamExt, TryStreamExt, stream};
use sqlx::{PgPool, Postgres};

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Address {
    pub user_id: String,
    pub state: Option<String>,
    pub country: Option<String>,
}

pub const FETCH_ADDRESS_QUERY: &str = r#"
    SELECT
        user_id,
        state,
        country
    FROM
        addresses
"#;

fn fetch_address_query<'builder>() -> sqlx::QueryBuilder<'builder, Postgres> {
    sqlx::QueryBuilder::new(FETCH_ADDRESS_QUERY)
}

pub async fn by_email(pool: &PgPool, email: &str) -> Result<Vec<Address>> {
    let brns = fetch_address_query()
        .push("WHERE user_id = ")
        .push_bind(user::id_for_email(email))
        .build_query_as::<Address>()
        .fetch_all(pool)
        .await?;

    Ok(brns)
}

pub async fn upsert_many(pool: &PgPool, addresses: &[Address]) -> Result<u64> {
    if addresses.is_empty() {
        return Ok(0);
    }
    let affected: Vec<u64> = stream::iter(addresses)
        .chunks(DB_INSERT_CHUNK_SIZE)
        .map(Ok)
        .and_then(|chunk| async move {
            let result = sqlx::QueryBuilder::new(
                r#"INSERT INTO addresses (
                    user_id,
                    state,
                    country
                ) "#,
            )
            .push_values(chunk, |mut b, address| {
                b.push_bind(&address.user_id)
                    .push_bind(&address.state)
                    .push_bind(&address.country);
            })
            .push(
                r#"ON CONFLICT(user_id) DO UPDATE SET
                state = excluded.state,
                country = excluded.country
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

pub async fn retain(pool: &PgPool, addresses: &[Address]) -> Result<u64> {
    retain_with_keys(pool, "addresses", "user_id", addresses, |address| {
        address.user_id.as_str()
    })
    .await
}
