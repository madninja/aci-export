pub type Result<T = ()> = anyhow::Result<T>;
pub type Error = anyhow::Error;
pub use anyhow::Context;

pub mod address;
pub mod brn;
pub mod club;
pub mod member;
pub mod region;
pub mod user;
