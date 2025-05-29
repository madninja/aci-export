pub type Result<T = ()> = anyhow::Result<T>;
pub type Error = anyhow::Error;
pub use anyhow::Context;

pub mod api;
pub mod cron;
pub mod server;
pub mod settings;
