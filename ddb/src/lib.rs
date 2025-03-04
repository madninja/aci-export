mod error;
pub use error::{Error, Result};

pub mod clubs;
pub mod members;
pub mod regions;
pub mod users;

/// A type alias for `Future` that may return `crate::error::Error`
pub type Future<'a, T> = futures::future::BoxFuture<'a, Result<T>>;

/// A type alias for `Stream` that may result in `crate::error::Error`
pub type Stream<'a, T> = futures::stream::BoxStream<'a, Result<T>>;

pub async fn connect(url: &str) -> Result<sqlx::MySqlPool> {
    use sqlx::{Executor, MySqlPool};
    let pool = MySqlPool::connect(url).await?;
    let _ = pool
        .execute(
            r#"
            SET GLOBAL table_definition_cache = 4096;
            SET GLOBAL table_open_cache = 4096;
        "#,
        )
        .await?;
    Ok(pool)
}
