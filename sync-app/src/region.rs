use crate::{Error, Result};
use ddb::regions;
use futures::TryFutureExt;
use sqlx::{PgExecutor, Postgres};

pub async fn all<'c, E>(exec: E) -> Result<Vec<Region>>
where
    E: PgExecutor<'c>,
{
    sqlx::query_as::<_, Region>(FETCH_REGIONS_QUERY)
        .fetch_all(exec)
        .map_err(Error::from)
        .await
}

pub async fn by_uid<'c, E>(exec: E, uid: i64) -> Result<Option<Region>>
where
    E: PgExecutor<'c>,
{
    let region = fetch_regions_query()
        .push("where uid = ")
        .push_bind(uid)
        .build_query_as::<Region>()
        .fetch_optional(exec)
        .await?;

    Ok(region)
}

pub async fn by_number<'c, E>(exec: E, number: i32) -> Result<Option<Region>>
where
    E: PgExecutor<'c>,
{
    let region = fetch_regions_query()
        .push("where number = ")
        .push_bind(number)
        .build_query_as::<Region>()
        .fetch_optional(exec)
        .await?;

    Ok(region)
}

pub async fn upsert_many<'c, E>(exec: E, regions: &[Region]) -> Result<u64>
where
    E: PgExecutor<'c>,
{
    if regions.is_empty() {
        return Ok(0);
    }
    let result = sqlx::QueryBuilder::new("INSERT INTO regions(uid, number, name) ")
        .push_values(regions, |mut b, region| {
            b.push_bind(&region.uid)
                .push_bind(&region.number)
                .push_bind(&region.name);
        })
        .push(
            r#"ON CONFLICT(number) DO UPDATE SET
                name = excluded.name,
                uid = excluded.uid
            "#,
        )
        .build()
        .execute(exec)
        .await?;
    Ok(result.rows_affected())
}

pub async fn retain<'c, E>(exec: E, regions: &[Region]) -> Result<u64>
where
    E: PgExecutor<'c>,
{
    if regions.is_empty() {
        return Ok(0);
    }
    let mut builder = sqlx::QueryBuilder::new(r#" DELETE FROM regions WHERE uid NOT IN ("#);
    let mut seperated = builder.separated(", ");
    for region in regions {
        seperated.push_bind(region.uid);
    }
    seperated.push_unseparated(") ");
    let result = builder.build().execute(exec).await?;
    Ok(result.rows_affected())
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

impl From<regions::Region> for Region {
    fn from(value: regions::Region) -> Self {
        Self {
            uid: value.uid as i64,
            number: value.number,
            name: value.name,
        }
    }
}
