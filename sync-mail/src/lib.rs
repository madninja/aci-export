pub type Result<T = ()> = anyhow::Result<T>;
pub type Error = anyhow::Error;
pub use anyhow::Context;

pub mod cmd;
pub mod mailchimp;
pub mod settings;
