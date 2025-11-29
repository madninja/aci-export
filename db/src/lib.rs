pub type Result<T = ()> = anyhow::Result<T>;
pub type Error = anyhow::Error;
pub use anyhow::Context;

pub mod address;
pub mod brn;
pub mod club;
pub mod member;
pub mod region;
pub mod user;

pub(crate) const DB_INSERT_CHUNK_SIZE: usize = 1000;
pub(crate) const DB_DELETE_CHUNK_SIZE: usize = 1000;

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
    let mut total_affected = 0;

    for chunk in items.chunks(DB_DELETE_CHUNK_SIZE) {
        let mut builder = sqlx::QueryBuilder::<sqlx::Postgres>::new("DELETE FROM ");
        builder.push(table);
        builder.push(" WHERE ");
        builder.push(column);
        builder.push(" NOT IN (");

        let mut separated = builder.separated(", ");
        for item in chunk {
            separated.push_bind(key_fn(item));
        }
        separated.push_unseparated(")");

        let result = builder.build().execute(&mut *tx).await?;
        total_affected += result.rows_affected();
    }

    tx.commit().await?;
    Ok(total_affected)
}
