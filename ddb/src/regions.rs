use crate::{Error, Result, Stream};
use futures::{StreamExt, TryStreamExt};
use sqlx::{MySql, MySqlPool};

pub fn all(exec: &MySqlPool) -> Stream<Region> {
    sqlx::query_as::<_, Region>(FETCH_REGIONS_QUERY)
        .fetch(exec)
        .map_err(Error::from)
        .boxed()
}

pub async fn by_uid(exec: &MySqlPool, uid: u64) -> Result<Option<Region>> {
    let region = fetch_regions_query()
        .push("where field_region_target_id = ")
        .push_bind(uid)
        .build_query_as::<Region>()
        .fetch_optional(exec)
        .await?;

    Ok(region)
}

pub async fn by_number(exec: &MySqlPool, number: i32) -> Result<Option<Region>> {
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
