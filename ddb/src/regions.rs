use crate::{Error, Result};
use futures::TryFutureExt;
use sqlx::{MySql, MySqlExecutor};

pub async fn all<'c, E>(exec: E) -> Result<Vec<Region>>
where
    E: MySqlExecutor<'c>,
{
    sqlx::query_as::<_, Region>(FETCH_REGIONS_QUERY)
        .fetch_all(exec)
        .map_err(Error::from)
        .await
}

pub async fn by_uid<'c, E>(exec: E, uid: u64) -> Result<Option<Region>>
where
    E: MySqlExecutor<'c>,
{
    let region = fetch_regions_query()
        .push("where field_region_target_id = ")
        .push_bind(uid)
        .build_query_as::<Region>()
        .fetch_optional(exec)
        .await?;

    Ok(region)
}

pub async fn by_number<'c, E>(exec: E, number: i32) -> Result<Option<Region>>
where
    E: MySqlExecutor<'c>,
{
    let region = fetch_regions_query()
        .push("where rn.field_region_number_value = ")
        .push_bind(number)
        .build_query_as::<Region>()
        .fetch_optional(exec)
        .await?;

    Ok(region)
}

const FETCH_REGIONS_QUERY: &str = r#"
        select
            region.entity_id as uid,  
            region.field_region_number_value as number,
            fields.title as name
        from node__field_region_number region
        inner join node_field_data fields on fields.nid = region.entity_id    
    "#;

fn fetch_regions_query<'builder>() -> sqlx::QueryBuilder<'builder, MySql> {
    sqlx::QueryBuilder::new(FETCH_REGIONS_QUERY)
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Region {
    pub uid: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}
