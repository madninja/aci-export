use crate::{Error, Result, Stream};
use futures::{StreamExt, TryStreamExt};
use sqlx::{MySql, MySqlPool};

pub fn all(exec: &MySqlPool) -> Stream<Club> {
    sqlx::query_as::<_, Club>(FETCH_CLUBS_QUERY)
        .fetch(exec)
        .map_err(Error::from)
        .boxed()
}

pub async fn by_uid(exec: &MySqlPool, uid: u64) -> Result<Option<Club>> {
    let club = fetch_clubs_query()
        .push("where field_club_target_id = ")
        .push_bind(uid)
        .build_query_as::<Club>()
        .fetch_optional(exec)
        .await?;

    Ok(club)
}

pub async fn by_number(exec: &MySqlPool, number: i32) -> Result<Option<Club>> {
    let club = fetch_clubs_query()
        .push("where cn.field_club_number_value = ")
        .push_bind(number)
        .build_query_as::<Club>()
        .fetch_optional(exec)
        .await?;

    Ok(club)
}

const FETCH_CLUBS_QUERY: &str = r#"
        select distinct
            field_club_target_id as uid,
            cn.field_club_number_value as number,
            nd.title as name,
            rn.field_region_number_value as region
        from paragraph__field_club pc
        left join node__field_club_number cn on cn.entity_id = pc.field_club_target_id 	
        left join node_field_data nd on nd.nid = pc.field_club_target_id
        inner join node__field_region nr on nr.entity_id = cn.entity_id
        inner join node__field_region_number rn on rn.entity_id = nr.field_region_target_id
    "#;

fn fetch_clubs_query<'builder>() -> sqlx::QueryBuilder<'builder, MySql> {
    sqlx::QueryBuilder::new(FETCH_CLUBS_QUERY)
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Club {
    pub uid: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub number: Option<i64>,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub name: String,
    pub region: i64,
}
