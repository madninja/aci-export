use crate::{Error, Result};
use futures::TryFutureExt;
use sqlx::MySqlPool;

/// Race taxonomy term from Drupal (vocabulary: ssp_race)
#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Race {
    /// Drupal taxonomy term ID (called uid for portal consistency)
    pub uid: u64,
    pub name: String,
}

/// Fetch all race taxonomy terms from Drupal
pub async fn all(pool: &MySqlPool) -> Result<Vec<Race>> {
    sqlx::query_as::<_, Race>(
        r#"
        SELECT tid AS uid, name
        FROM taxonomy_term_field_data
        WHERE vid = 'ssp_race'
        ORDER BY tid
        "#,
    )
    .fetch_all(pool)
    .map_err(Error::from)
    .await
}
