use crate::{Error, Result, retain_with_keys};
use futures::TryFutureExt;
use sqlx::{PgPool, Postgres};

pub async fn all(pool: &PgPool) -> Result<Vec<Region>> {
    sqlx::query_as::<_, Region>(FETCH_REGIONS_QUERY)
        .fetch_all(pool)
        .map_err(Error::from)
        .await
}

pub async fn by_uid(pool: &PgPool, uid: i64) -> Result<Option<Region>> {
    let region = fetch_regions_query()
        .push("where uid = ")
        .push_bind(uid)
        .build_query_as::<Region>()
        .fetch_optional(pool)
        .await?;

    Ok(region)
}

pub async fn by_number(pool: &PgPool, number: i32) -> Result<Option<Region>> {
    let region = fetch_regions_query()
        .push("where number = ")
        .push_bind(number)
        .build_query_as::<Region>()
        .fetch_optional(pool)
        .await?;

    Ok(region)
}

pub async fn upsert_many(pool: &PgPool, regions: &[Region]) -> Result<u64> {
    if regions.is_empty() {
        return Ok(0);
    }
    let result = sqlx::QueryBuilder::new("INSERT INTO regions(uid, number, name) ")
        .push_values(regions, |mut b, region| {
            b.push_bind(region.uid)
                .push_bind(region.number)
                .push_bind(&region.name);
        })
        .push(
            r#"ON CONFLICT(number) DO UPDATE SET
                name = excluded.name,
                uid = excluded.uid
            "#,
        )
        .build()
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn retain(pool: &PgPool, regions: &[Region]) -> Result<u64> {
    retain_with_keys(pool, "regions", "uid", regions, |region| region.uid).await
}

const FETCH_REGIONS_QUERY: &str = r#"
        select
            uid,
            number,
            name
        from regions
    "#;

fn fetch_regions_query<'builder>() -> sqlx::QueryBuilder<'builder, Postgres> {
    sqlx::QueryBuilder::new(FETCH_REGIONS_QUERY)
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Region {
    pub uid: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}
