use crate::{Result, cmd::print_json, settings::Settings, sync};

/// Run the app database sync from the membership database
#[derive(Debug, clap::Args)]
pub struct Cmd {}

impl Cmd {
    pub async fn run(&self, settings: Settings) -> Result {
        let stats = sync::run(&settings.app, &settings.ddb).await?;
        print_json(&stats)
    }
}
