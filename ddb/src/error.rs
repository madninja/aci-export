use thiserror::Error;

pub type Result<T = ()> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("database: {0}")]
    Request(#[from] sqlx::Error),
}

impl Error {}
