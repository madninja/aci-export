//! User roles and microsite admin assignments from Drupal.

use crate::{Error, Result};
use futures::TryFutureExt;
use sqlx::MySqlPool;

/// User role assignment from Drupal's user__roles table
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct UserRole {
    pub user_uid: u64,
    pub role: String,
}

/// Microsite admin assignment linking a user to a club or region.
/// The entity_uid is the nid of the actual ssp_club or ssp_region node (not the microsite_homepage).
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct MicrositeAdmin {
    pub user_uid: u64,
    /// The nid of the ssp_club or ssp_region node (matches portal's legacy_uid)
    pub entity_uid: u64,
    /// True if this is a region admin, false if club admin
    pub is_region: bool,
}

/// Fetch all user role assignments from Drupal
pub async fn all(pool: &MySqlPool) -> Result<Vec<UserRole>> {
    sqlx::query_as::<_, UserRole>(
        r#"
        SELECT entity_id AS user_uid, roles_target_id AS role
        FROM user__roles
        WHERE deleted = 0
        "#,
    )
    .fetch_all(pool)
    .map_err(Error::from)
    .await
}

/// Fetch all microsite admin assignments, resolving to actual ssp_club/ssp_region nids.
///
/// This query joins via `field_main_site_club` which links ssp_club/ssp_region
/// nodes to their corresponding microsite_homepage (no title matching needed).
pub async fn microsite_admins(pool: &MySqlPool) -> Result<Vec<MicrositeAdmin>> {
    sqlx::query_as::<_, MicrositeAdmin>(
        r#"
        SELECT
            uf.entity_id AS user_uid,
            COALESCE(region_link.entity_id, club_link.entity_id) AS entity_uid,
            (region_link.entity_id IS NOT NULL) AS is_region
        FROM user__field_microsite uf
        LEFT JOIN node__field_main_site_club club_link
            ON club_link.field_main_site_club_target_id = uf.field_microsite_target_id
            AND club_link.bundle = 'ssp_club'
            AND club_link.deleted = 0
        LEFT JOIN node__field_main_site_club region_link
            ON region_link.field_main_site_club_target_id = uf.field_microsite_target_id
            AND region_link.bundle = 'ssp_region'
            AND region_link.deleted = 0
        WHERE uf.deleted = 0
          AND (club_link.entity_id IS NOT NULL OR region_link.entity_id IS NOT NULL)
        "#,
    )
    .fetch_all(pool)
    .map_err(Error::from)
    .await
}
