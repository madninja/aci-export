use crate::{users::User, Error, Result};
use sqlx::{MySql, MySqlPool};

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Leadership {
    pub entity_uid: u64,
    #[sqlx(flatten, try_from = "RoleFromRow")]
    pub role: Role,
    pub start_date: chrono::NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<chrono::NaiveDate>,
    #[sqlx(flatten)]
    pub user: User,
}

#[derive(Debug, serde::Serialize)]
pub struct Role {
    pub uid: u64,
    pub title: String,
}

// Intermediary struct for sqlx extraction (similar to PartnerUser pattern in members.rs)
#[derive(Debug, sqlx::FromRow)]
struct RoleFromRow {
    role_uid: u64,
    role_title: String,
}

impl From<RoleFromRow> for Role {
    fn from(value: RoleFromRow) -> Self {
        Self {
            uid: value.role_uid,
            title: value.role_title,
        }
    }
}

const FETCH_LEADERSHIP_QUERY: &str = r#"
    SELECT
        entity.nid AS entity_uid,
        role_term.tid AS role_uid,
        role_term.name AS role_title,
        DATE(start.field_start_date_value) AS start_date,
        DATE(end.field_end_date_value) AS end_date,
        usr.uid AS uid,
        COALESCE(md.email, usr.mail) AS email,
        ufn.field_first_name_value AS first_name,
        uln.field_last_name_value AS last_name,
        CAST(ubd.field_birth_date_value AS DATE) AS birthday,
        DATE(FROM_UNIXTIME(usr.login)) AS last_login
    FROM node_field_data entity
    JOIN node__field_leadership_ssp l
        ON l.entity_id = entity.nid AND l.deleted = '0'
    JOIN paragraphs_item_field_data p
        ON p.id = l.field_leadership_ssp_target_id
    LEFT JOIN paragraph__field_role r ON r.entity_id = p.id AND r.deleted = '0'
    LEFT JOIN taxonomy_term_field_data role_term ON role_term.tid = r.field_role_target_id
    LEFT JOIN paragraph__field_start_date start ON start.entity_id = p.id AND start.deleted = '0'
    LEFT JOIN paragraph__field_end_date end ON end.entity_id = p.id AND end.deleted = '0'
    LEFT JOIN paragraph__field_user u ON u.entity_id = p.id AND u.deleted = '0'
    LEFT JOIN paragraph__field_member m ON m.entity_id = p.id AND m.deleted = '0'
    JOIN users_field_data usr ON usr.uid = COALESCE(u.field_user_target_id, m.field_member_target_id)
    LEFT JOIN z_member_search_main md ON md.user_id = usr.uid
    LEFT JOIN user__field_first_name ufn ON ufn.entity_id = usr.uid
    LEFT JOIN user__field_last_name uln ON uln.entity_id = usr.uid
    LEFT JOIN user__field_birth_date ubd ON ubd.entity_id = usr.uid
    WHERE DATE(start.field_start_date_value) <= CURRENT_DATE
      AND (end.field_end_date_value IS NULL OR DATE(end.field_end_date_value) >= CURRENT_DATE)
      AND
"#;

fn fetch_leadership_query<'builder>() -> sqlx::QueryBuilder<'builder, MySql> {
    sqlx::QueryBuilder::new(FETCH_LEADERSHIP_QUERY)
}

async fn fetch_leadership_for_type(
    pool: &MySqlPool,
    entity_type: &str,
    entity_id: Option<u64>,
) -> Result<Vec<Leadership>> {
    use futures::TryFutureExt;

    let mut query = fetch_leadership_query();

    if let Some(id) = entity_id {
        query.push("entity.nid = ").push_bind(id).push(" AND ");
    }

    query.push("entity.type = ").push_bind(entity_type);

    query
        .build_query_as::<Leadership>()
        .fetch_all(pool)
        .map_err(Error::from)
        .await
}

pub async fn for_club(pool: &MySqlPool, uid: u64) -> Result<Vec<Leadership>> {
    fetch_leadership_for_type(pool, "ssp_club", Some(uid)).await
}

pub async fn for_all_clubs(pool: &MySqlPool) -> Result<Vec<Leadership>> {
    fetch_leadership_for_type(pool, "ssp_club", None).await
}

pub async fn for_region(pool: &MySqlPool, uid: u64) -> Result<Vec<Leadership>> {
    fetch_leadership_for_type(pool, "ssp_region", Some(uid)).await
}

pub async fn for_all_regions(pool: &MySqlPool) -> Result<Vec<Leadership>> {
    fetch_leadership_for_type(pool, "ssp_region", None).await
}

pub async fn for_club_by_number(pool: &MySqlPool, number: i32) -> Result<Vec<Leadership>> {
    // First lookup club by number to get uid
    let club = crate::clubs::by_number(pool, number)
        .await?
        .ok_or_else(|| Error::Request(sqlx::Error::RowNotFound))?;

    // Then get leadership for that club uid
    for_club(pool, club.uid).await
}

pub async fn for_region_by_number(pool: &MySqlPool, number: i32) -> Result<Vec<Leadership>> {
    // First lookup region by number to get uid
    let region = crate::regions::by_number(pool, number)
        .await?
        .ok_or_else(|| Error::Request(sqlx::Error::RowNotFound))?;

    // Then get leadership for that region uid
    for_region(pool, region.uid).await
}

pub async fn for_international(pool: &MySqlPool) -> Result<Vec<Leadership>> {
    fetch_leadership_for_type(pool, "ssp_international_leadership", None).await
}
