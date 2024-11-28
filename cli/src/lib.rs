pub type Result<T = ()> = anyhow::Result<T>;
pub type Error = anyhow::Error;

pub mod cmd;
pub mod settings;
