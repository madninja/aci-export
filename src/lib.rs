pub type Result<T = ()> = anyhow::Result<T>;
pub type Error = anyhow::Error;

pub mod cmd;
pub use db::member;
pub mod settings;
pub use db::user;
