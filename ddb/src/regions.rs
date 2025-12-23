use crate::{Error, Result};
use futures::TryFutureExt;
use sqlx::{MySql, MySqlPool};

pub async fn all(pool: &MySqlPool) -> Result<Vec<Region>> {
    sqlx::query_as::<_, Region>(FETCH_REGIONS_QUERY)
        .fetch_all(pool)
        .map_err(Error::from)
        .await
}

pub async fn by_uid(pool: &MySqlPool, uid: u64) -> Result<Option<Region>> {
    let region = fetch_regions_query()
        .push("where region.entity_id = ")
        .push_bind(uid)
        .build_query_as::<Region>()
        .fetch_optional(pool)
        .await?;

    Ok(region)
}

pub async fn by_number(pool: &MySqlPool, number: i32) -> Result<Option<Region>> {
    let region = fetch_regions_query()
        .push("where region.field_region_number_value = ")
        .push_bind(number)
        .build_query_as::<Region>()
        .fetch_optional(pool)
        .await?;

    Ok(region)
}

const FETCH_REGIONS_QUERY: &str = r#"
        select
            region.entity_id as uid,
            region.field_region_number_value as number,
            fields.title as name,
            fields.status as active
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
    pub active: bool,
}

pub mod db {
    use super::*;
    use ::db as app_db;

    impl From<Region> for app_db::region::Region {
        fn from(value: Region) -> Self {
            Self {
                uid: value.uid as i64,
                number: value.number,
                name: value.name,
            }
        }
    }

    impl From<crate::leadership::Leadership> for app_db::region::Leadership {
        fn from(value: crate::leadership::Leadership) -> Self {
            Self {
                id: None,
                region: app_db::region::Region {
                    uid: value.entity_uid as i64,
                    number: None,
                    name: None,
                },
                user: value.user.into(),
                role: value.role.into(),
                start_date: value.start_date,
                end_date: value.end_date,
            }
        }
    }
}
