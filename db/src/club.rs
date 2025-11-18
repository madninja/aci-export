use crate::{Error, Result};
use futures::TryFutureExt;
use sqlx::{PgExecutor, Postgres};

pub async fn all<'c, E>(exec: E) -> Result<Vec<Club>>
where
    E: PgExecutor<'c>,
{
    sqlx::query_as::<_, Club>(FETCH_CLUBS_QUERY)
        .fetch_all(exec)
        .map_err(Error::from)
        .await
}

pub async fn by_uid<'c, E>(exec: E, uid: i64) -> Result<Option<Club>>
where
    E: PgExecutor<'c>,
{
    let club = fetch_clubs_query()
        .push("where uid = ")
        .push_bind(uid)
        .build_query_as::<Club>()
        .fetch_optional(exec)
        .await?;

    Ok(club)
}

pub async fn by_number<'c, E>(exec: E, number: i32) -> Result<Option<Club>>
where
    E: PgExecutor<'c>,
{
    let club = fetch_clubs_query()
        .push("where number = ")
        .push_bind(number)
        .build_query_as::<Club>()
        .fetch_optional(exec)
        .await?;

    Ok(club)
}

const FETCH_CLUBS_QUERY: &str = r#"
    SELECT
        uid,
        number,
        name,
        region
    FROM
        clubs
"#;

fn fetch_clubs_query<'builder>() -> sqlx::QueryBuilder<'builder, Postgres> {
    sqlx::QueryBuilder::new(FETCH_CLUBS_QUERY)
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Club {
    pub uid: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<i64>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<i64>,
}

pub async fn upsert_many<'c, E>(exec: E, clubs: &[Club]) -> Result<u64>
where
    E: PgExecutor<'c>,
{
    if clubs.is_empty() {
        return Ok(0);
    }
    let result = sqlx::QueryBuilder::new("INSERT INTO clubs(uid, number, name, region) ")
        .push_values(clubs, |mut b, club| {
            b.push_bind(club.uid)
                .push_bind(club.number)
                .push_bind(&club.name)
                .push_bind(club.region);
        })
        .push(
            r#"ON CONFLICT(number) DO UPDATE SET
                name = excluded.name,
                uid = excluded.uid,
                region = excluded.region
            "#,
        )
        .build()
        .execute(exec)
        .await?;
    Ok(result.rows_affected())
}

pub async fn retain<'c, E>(exec: E, clubs: &[Club]) -> Result<u64>
where
    E: PgExecutor<'c>,
{
    if clubs.is_empty() {
        return Ok(0);
    }
    let mut builder = sqlx::QueryBuilder::new(r#" DELETE FROM clubs WHERE uid NOT IN ("#);
    let mut seperated = builder.separated(", ");
    for region in clubs {
        seperated.push_bind(region.uid);
    }
    seperated.push_unseparated(") ");
    let result = builder.build().execute(exec).await?;
    Ok(result.rows_affected())
}
