use crate::{settings::Settings, Result};

#[derive(Debug, clap::Args)]
pub struct Cmd {}

impl Cmd {
    pub async fn run(&self, settings: &Settings) -> Result {
        let _pool = settings.database.connect().await?;
        Ok(())
    }
}
