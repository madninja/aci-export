mod error;
pub use error::{Error, Result};

pub mod clubs;
pub mod members;
pub mod users;

/// A type alias for `Future` that may return `crate::error::Error`
pub type Future<'a, T> = futures::future::BoxFuture<'a, Result<T>>;

/// A type alias for `Stream` that may result in `crate::error::Error`
pub type Stream<'a, T> = futures::stream::BoxStream<'a, Result<T>>;
