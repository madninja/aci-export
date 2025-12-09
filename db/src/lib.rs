pub type Result<T = ()> = anyhow::Result<T>;
pub type Error = anyhow::Error;
pub use anyhow::Context;

pub mod address;
pub mod brn;
pub mod club;
pub mod leadership;
pub mod member;
pub mod region;
pub mod user;

pub(crate) const DB_INSERT_CHUNK_SIZE: usize = 1000;

pub(crate) async fn retain_with_keys<'a, T, F, K>(
    pool: &sqlx::PgPool,
    table: &str,
    column: &str,
    items: &'a [T],
    mut key_fn: F,
) -> Result<u64>
where
    F: FnMut(&'a T) -> K,
    for<'q> K: sqlx::Encode<'q, sqlx::Postgres> + sqlx::Type<sqlx::Postgres>,
{
    if items.is_empty() {
        return Ok(0);
    }

    let mut tx = pool.begin().await?;

    // Create temporary table with unique name (timestamp-based)
    let temp_table = format!(
        "temp_retain_{table}_{timestamp}",
        table = table,
        timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    );

    // Use TEXT column for simplicity (works with all key types via casting)
    sqlx::query(&format!(
        "CREATE TEMPORARY TABLE {temp_table} (retain_key TEXT NOT NULL) ON COMMIT DROP"
    ))
    .execute(&mut *tx)
    .await?;

    // Insert keys in chunks for efficiency
    for chunk in items.chunks(DB_INSERT_CHUNK_SIZE) {
        let mut builder = sqlx::QueryBuilder::<sqlx::Postgres>::new(format!(
            "INSERT INTO {temp_table} (retain_key) "
        ));

        builder.push_values(chunk, |mut b, item| {
            b.push_bind(key_fn(item));
        });

        builder.build().execute(&mut *tx).await?;
    }

    // Delete rows not in temporary table (cast for comparison)
    let delete_query = format!(
        "DELETE FROM {table} WHERE {column}::TEXT NOT IN (SELECT retain_key FROM {temp_table})"
    );

    let result = sqlx::query(&delete_query).execute(&mut *tx).await?;

    let total_affected = result.rows_affected();

    // Commit (temp table automatically drops)
    tx.commit().await?;

    Ok(total_affected)
}
