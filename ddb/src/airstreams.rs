//! Airstream queries from Drupal database
//!
//! Returns all ownership records (paragraphs) with full date tracking.
//! Each ownership paragraph links a user to an airstream with join/leave dates.

use crate::Result;
use chrono::NaiveDate;
use sqlx::{mysql::MySql, MySqlPool};

/// Airstream ownership record from Drupal database
/// Each row represents an ownership period (paragraph entity)
#[derive(Debug, sqlx::FromRow, serde::Serialize, Clone)]
pub struct Airstream {
    /// Drupal node ID (nid) - identifies the airstream vehicle
    pub airstream_id: u64,
    /// Drupal paragraph ID - identifies this ownership record
    pub paragraph_id: u64,
    /// User UID who owns/owned this airstream
    pub user_id: u64,
    /// Include partner in ownership
    pub include_partner: bool,
    /// Ownership start date (required in Drupal)
    pub join_date: NaiveDate,
    /// Ownership end date (NULL = current owner)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub leave_date: Option<NaiveDate>,
    // Vehicle details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Type: "Trailer", "Class A", "Class B"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub airstream_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<i32>,
    /// Length in feet (MySQL DECIMAL maps to String for precision)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length: Option<String>,
}

fn fetch_airstream_query<'builder>() -> sqlx::QueryBuilder<'builder, MySql> {
    // Return all ownership paragraphs with their dates
    // An airstream may have multiple ownership records (current + historical)
    sqlx::QueryBuilder::new(
        r#"
            SELECT
                n.nid AS airstream_id,
                p.id AS paragraph_id,
                m.field_member_target_id AS user_id,
                COALESCE(inc.field_include_partner_member_value, 0) = 1 AS include_partner,
                CAST(jd.field_join_date_value AS DATE) AS join_date,
                CAST(ld.field_leave_date_value AS DATE) AS leave_date,
                n.title AS vin,
                model.field_airstream_model_value AS model,
                rig.field_rig_type_value AS airstream_type,
                year.field_airstream_year_value AS year,
                CAST(length.field_airstream_length_value AS CHAR) AS length
            FROM node_field_data n
            -- Airstream details
            LEFT JOIN node__field_airstream_model model ON n.nid = model.entity_id AND model.deleted = 0
            LEFT JOIN node__field_rig_type rig ON n.nid = rig.entity_id AND rig.deleted = 0
            LEFT JOIN node__field_airstream_year year ON n.nid = year.entity_id AND year.deleted = 0
            LEFT JOIN node__field_airstream_length length ON n.nid = length.entity_id AND length.deleted = 0
            -- Ownership paragraph
            LEFT JOIN node__field_ownership own ON n.nid = own.entity_id AND own.deleted = 0
            LEFT JOIN paragraphs_item_field_data p ON p.id = own.field_ownership_target_id AND p.status = 1
            -- Ownership fields
            LEFT JOIN paragraph__field_member m ON p.id = m.entity_id AND m.deleted = 0
            LEFT JOIN paragraph__field_include_partner_member inc ON p.id = inc.entity_id AND inc.deleted = 0
            LEFT JOIN paragraph__field_join_date jd ON p.id = jd.entity_id AND jd.deleted = 0
            LEFT JOIN paragraph__field_leave_date ld ON p.id = ld.entity_id AND ld.deleted = 0
            WHERE n.type = 'ssp_airstream'
              AND m.field_member_target_id IS NOT NULL
              AND jd.field_join_date_value IS NOT NULL
        "#,
    )
}

/// Fetch all airstream ownership records from Drupal
pub async fn all(pool: &MySqlPool) -> Result<Vec<Airstream>> {
    let airstreams = fetch_airstream_query()
        .build_query_as::<Airstream>()
        .fetch_all(pool)
        .await?;

    Ok(airstreams)
}

/// Fetch airstream ownership records for a specific user
pub async fn by_user_id(pool: &MySqlPool, user_id: u64) -> Result<Vec<Airstream>> {
    let airstreams = fetch_airstream_query()
        .push(" AND m.field_member_target_id = ")
        .push_bind(user_id)
        .build_query_as::<Airstream>()
        .fetch_all(pool)
        .await?;

    Ok(airstreams)
}
