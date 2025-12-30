//! User address queries from Drupal database
//!
//! Returns all addresses for users as paragraph entities.
//! Each user can have multiple addresses with primary/mailing flags.

use crate::Result;
use sqlx::{MySqlPool, mysql::MySql};

/// User address record from Drupal database
/// Each row represents an address paragraph entity
#[derive(Debug, sqlx::FromRow, serde::Serialize, Clone)]
pub struct Address {
    /// Drupal paragraph ID - identifies this address
    pub paragraph_id: u64,
    /// User UID who owns this address
    pub user_uid: u64,
    /// Ordering within user's addresses (0 = first)
    pub delta: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street_address: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street_address_2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zip_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
    /// Is this the user's primary address?
    pub is_primary: bool,
    /// Should this address be used for mailing?
    pub is_mailing_address: bool,
}

fn fetch_address_query<'builder>() -> sqlx::QueryBuilder<'builder, MySql> {
    sqlx::QueryBuilder::new(
        r#"
            SELECT
                ua.field_address_target_id AS paragraph_id,
                ua.entity_id AS user_uid,
                ua.delta,
                addr.field_address_value AS street_address,
                addr2.field_street_address_2_value AS street_address_2,
                city.field_city_value AS city,
                state.field_state_name_value AS state,
                zip.field_zip_code_value AS zip_code,
                country.field_country_value AS country,
                COALESCE(prim.field_primary_address_value, 0) = 1 AS is_primary,
                COALESCE(mail.field_use_as_mailing_address_value, 0) = 1 AS is_mailing_address
            FROM user__field_address ua
            LEFT JOIN paragraph__field_address addr
                ON ua.field_address_target_id = addr.entity_id AND addr.deleted = 0
            LEFT JOIN paragraph__field_street_address_2 addr2
                ON ua.field_address_target_id = addr2.entity_id AND addr2.deleted = 0
            LEFT JOIN paragraph__field_city city
                ON ua.field_address_target_id = city.entity_id AND city.deleted = 0
            LEFT JOIN paragraph__field_state_name state
                ON ua.field_address_target_id = state.entity_id AND state.deleted = 0
            LEFT JOIN paragraph__field_zip_code zip
                ON ua.field_address_target_id = zip.entity_id AND zip.deleted = 0
            LEFT JOIN paragraph__field_country country
                ON ua.field_address_target_id = country.entity_id AND country.deleted = 0
            LEFT JOIN paragraph__field_primary_address prim
                ON ua.field_address_target_id = prim.entity_id AND prim.deleted = 0
            LEFT JOIN paragraph__field_use_as_mailing_address mail
                ON ua.field_address_target_id = mail.entity_id AND mail.deleted = 0
            WHERE ua.deleted = 0
        "#,
    )
}

/// Fetch all addresses from Drupal
pub async fn all(pool: &MySqlPool) -> Result<Vec<Address>> {
    fetch_address_query()
        .push(" ORDER BY ua.entity_id, ua.delta")
        .build_query_as::<Address>()
        .fetch_all(pool)
        .await
        .map_err(Into::into)
}

/// Fetch addresses for a specific user
pub async fn by_user_id(pool: &MySqlPool, user_uid: u64) -> Result<Vec<Address>> {
    fetch_address_query()
        .push(" AND ua.entity_id = ")
        .push_bind(user_uid)
        .push(" ORDER BY ua.delta")
        .build_query_as::<Address>()
        .fetch_all(pool)
        .await
        .map_err(Into::into)
}
