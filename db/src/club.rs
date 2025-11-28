use crate::{DB_DELETE_CHUNK_SIZE, Error, Result};
use futures::TryFutureExt;
use sqlx::{PgPool, Postgres};

pub async fn all(pool: &PgPool) -> Result<Vec<Club>> {
    sqlx::query_as::<_, Club>(FETCH_CLUBS_QUERY)
        .fetch_all(pool)
        .map_err(Error::from)
        .await
}

pub async fn by_uid(pool: &PgPool, uid: i64) -> Result<Option<Club>> {
    let club = fetch_clubs_query()
        .push("where uid = ")
        .push_bind(uid)
        .build_query_as::<Club>()
        .fetch_optional(pool)
        .await?;

    Ok(club)
}

pub async fn by_number(pool: &PgPool, number: i32) -> Result<Option<Club>> {
    let club = fetch_clubs_query()
        .push("where number = ")
        .push_bind(number)
        .build_query_as::<Club>()
        .fetch_optional(pool)
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

pub async fn upsert_many(pool: &PgPool, clubs: &[Club]) -> Result<u64> {
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
            r#"ON CONFLICT(uid) DO UPDATE SET
                name = excluded.name,
                number = excluded.number,
                region = excluded.region
            "#,
        )
        .build()
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn retain(pool: &PgPool, clubs: &[Club]) -> Result<u64> {
    if clubs.is_empty() {
        return Ok(0);
    }
    let uids: Vec<i64> = clubs.iter().map(|club| club.uid).collect();
    let mut tx = pool.begin().await?;
    let mut total_affected = 0;
    for chunk in uids.chunks(DB_DELETE_CHUNK_SIZE) {
        let mut builder = sqlx::QueryBuilder::new(r#" DELETE FROM clubs WHERE uid NOT IN ("#);
        let mut seperated = builder.separated(", ");
        for uid in chunk {
            seperated.push_bind(uid);
        }
        seperated.push_unseparated(") ");
        let result = builder.build().execute(&mut *tx).await?;
        total_affected += result.rows_affected();
    }
    tx.commit().await?;
    Ok(total_affected)
}
