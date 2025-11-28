use crate::{Context, Result, settings::Settings};

/// Database migration commands
#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[clap(subcommand)]
    cmd: MigrateCmd,
}

#[derive(Debug, clap::Subcommand)]
enum MigrateCmd {
    /// Run pending migrations
    Run(RunCmd),
    /// Show migration status
    Info(InfoCmd),
    /// Create a new migration file
    New(NewCmd),
}

impl Cmd {
    pub async fn run(&self, settings: Settings) -> Result {
        match &self.cmd {
            MigrateCmd::Run(cmd) => cmd.run(settings).await,
            MigrateCmd::Info(cmd) => cmd.run(settings).await,
            MigrateCmd::New(cmd) => cmd.run().await,
        }
    }
}

/// Run pending migrations
#[derive(Debug, clap::Args)]
struct RunCmd {}

impl RunCmd {
    async fn run(&self, settings: Settings) -> Result {
        let db = settings.mail.db.connect().await?;
        tracing::info!("running migrations");
        sqlx::migrate!()
            .run(&db)
            .await
            .context("running migrations")?;
        tracing::info!("migrations completed successfully");
        Ok(())
    }
}

/// Show migration status
#[derive(Debug, clap::Args)]
struct InfoCmd {}

impl InfoCmd {
    async fn run(&self, settings: Settings) -> Result {
        let db = settings.mail.db.connect().await?;
        let migrator = sqlx::migrate!();

        // Query applied migrations from the database
        let applied: Vec<(i64, String)> =
            sqlx::query_as("SELECT version, description FROM _sqlx_migrations ORDER BY version")
                .fetch_all(&db)
                .await
                .context("querying applied migrations")?;

        // Get all available migrations from the migrator
        let available = migrator.migrations;

        // Print applied migrations
        for (version, description) in &applied {
            println!("Applied {version}/{description}");
        }

        // Find and print pending migrations
        let applied_versions: std::collections::HashSet<i64> =
            applied.iter().map(|(v, _)| *v).collect();

        let pending: Vec<_> = available
            .iter()
            .filter(|m| !applied_versions.contains(&m.version))
            .collect();

        for migration in pending {
            println!("Pending {}/{}", migration.version, migration.description);
        }

        Ok(())
    }
}

/// Create a new migration file
#[derive(Debug, clap::Args)]
struct NewCmd {
    /// Description for the migration
    description: String,
}

impl NewCmd {
    async fn run(&self) -> Result {
        let timestamp = chrono::Utc::now().format("%Y%m%d%H%M%S");
        let description = self
            .description
            .replace(' ', "_")
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == '_')
            .collect::<String>()
            .to_lowercase();

        let filename = format!("{timestamp}_{description}.sql");
        let migrations_dir = std::env::current_dir()?.join("migrations");
        let filepath = migrations_dir.join(&filename);

        std::fs::write(&filepath, "")?;

        println!("Created {}", filepath.display());
        println!("\nNote: You must rebuild the application for this migration to be included.");

        Ok(())
    }
}
