use crate::{user, Error, Result};
use futures::{stream, StreamExt, TryStreamExt};
use sqlx::{postgres::PgExecutor, Postgres};

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Address {
    pub user_id: String,
    pub street_address: Option<String>,
    pub street_address_2: Option<String>,
    pub zip_code: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub country: Option<String>,
}

impl Address {
    pub fn from_member(member: &ddb::members::Member, address: ddb::members::Address) -> Self {
        Self {
            user_id: user::id_for_email(&member.primary.email),
            street_address: address.street_address,
            street_address_2: address.street_address_2,
            zip_code: address.zip_code,
            city: address.city,
            state: address.state,
            country: address.country,
        }
    }
}

pub const FETCH_ADDRESS_QUERY: &str = r#"
    SELECT
        user_id,
        street_address,
        street_address_2,
        zip_code,
        city,
        state,
        country
    FROM
        addresses
"#;

fn fetch_address_query<'builder>() -> sqlx::QueryBuilder<'builder, Postgres> {
    sqlx::QueryBuilder::new(FETCH_ADDRESS_QUERY)
}

pub async fn by_email<'c, E>(exec: E, email: &str) -> Result<Vec<Address>>
where
    E: PgExecutor<'c>,
{
    let brns = fetch_address_query()
        .push("WHERE user_id = ")
        .push_bind(user::id_for_email(email))
        .build_query_as::<Address>()
        .fetch_all(exec)
        .await?;

    Ok(brns)
}
pub async fn upsert_many<'c, E>(exec: E, addresses: &[Address]) -> Result<u64>
where
    E: PgExecutor<'c> + Copy,
{
    if addresses.is_empty() {
        return Ok(0);
    }
    let affected: Vec<u64> = stream::iter(addresses)
        .chunks(5000)
        .map(Ok)
        .and_then(|chunk| async move {
            let result = sqlx::QueryBuilder::new(
                r#"INSERT INTO addresses (
                    user_id,
                    street_address,
                    street_address_2,
                    zip_code,
                    city,
                    state,
                    country
                ) "#,
            )
            .push_values(chunk, |mut b, address| {
                b.push_bind(&address.user_id)
                    .push_bind(&address.street_address)
                    .push_bind(&address.street_address_2)
                    .push_bind(&address.zip_code)
                    .push_bind(&address.city)
                    .push_bind(&address.state)
                    .push_bind(&address.country);
            })
            .push(
                r#"ON CONFLICT(user_id) DO UPDATE SET
                street_address = excluded.street_address,
                street_address_2 = excluded.street_address_2,
                zip_code = excluded.zip_code,
                city = excluded.city,
                state = excluded.state,
                country = excluded.country
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

pub async fn retain<'c, E>(exec: E, addresses: &[Address]) -> Result<u64>
where
    E: PgExecutor<'c>,
{
    if addresses.is_empty() {
        return Ok(0);
    }
    let mut builder = sqlx::QueryBuilder::new(r#" DELETE FROM addresses WHERE user_id NOT IN ("#);
    let mut seperated = builder.separated(", ");
    for address in addresses {
        seperated.push_bind(&address.user_id);
    }
    seperated.push_unseparated(") ");
    let result = builder.build().execute(exec).await?;
    Ok(result.rows_affected())
}
