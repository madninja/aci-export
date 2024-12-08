use crate::{settings::AciDatabaseSettings, Error, Result};
use chrono::{DateTime, Utc};
use futures::TryFutureExt;
use tokio_cron_scheduler::{Job as CronJob, JobScheduler};

#[derive(Debug, sqlx::FromRow, Clone, serde::Serialize, Default)]
pub struct Job {
    pub id: i64,
    pub name: String,
    pub api_key: String,
    pub list: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub club: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<i32>,
    pub created_at: DateTime<Utc>,
}

impl Job {
    pub async fn schedule<'c, E>(
        db: E,
        ddb_settings: &AciDatabaseSettings,
        scheduler: &mut JobScheduler,
    ) -> Result
    where
        E: sqlx::Executor<'c, Database = sqlx::Postgres>,
    {
        let job_descriptors = Self::all(db).await?;
        for job in job_descriptors {
            scheduler.add(job.to_job(ddb_settings.clone())?).await?;
        }
        Ok(())
    }

    fn to_job(&self, ddb_url: AciDatabaseSettings) -> Result<CronJob> {
        CronJob::new_async("@daily", {
            let inner = self.clone();
            move |_uuid, _lock| {
                Box::pin({
                    let job = inner.clone();
                    let job_ddb_url = ddb_url.clone();
                    let job_name = job.name.clone();
                    let job_id = job.id;
                    async move {
                        if let Err(err) = job.sync(job_ddb_url).await {
                            tracing::error!(
                                ?err,
                                id = job_id,
                                name = job_name,
                                "failed to sync mailchimp"
                            );
                        }
                    }
                })
            }
        })
        .map_err(Error::from)
    }

    pub async fn all<'c, E>(db: E) -> Result<Vec<Self>>
    where
        E: sqlx::Executor<'c, Database = sqlx::Postgres>,
    {
        sqlx::query_as("select id, name, api_key, list, club, region, created_at from mailchimp")
            .fetch_all(db)
            .map_err(Error::from)
            .await
    }

    pub async fn get<'c, E>(db: E, job_id: i64) -> Result<Option<Self>>
    where
        E: sqlx::Executor<'c, Database = sqlx::Postgres>,
    {
        sqlx::query_as(
            r#"select id, name, api_key, list, club, region, created_at from mailchimp where id = $1;"#,
        )
        .bind(job_id)
        .fetch_optional(db)
        .map_err(Error::from)
        .await
    }

    pub async fn create<'c, E>(db: E, job: &Self) -> Result<Self>
    where
        E: sqlx::Executor<'c, Database = sqlx::Postgres>,
    {
        sqlx::query_as(
            r#"
            insert into mailchimp (mame, api_key, list, club, region)
            values ($1, $2, $4, $4, $5)
            returning *;
            "#,
        )
        .bind(&job.name)
        .bind(&job.api_key)
        .bind(&job.list)
        .bind(job.club)
        .bind(job.region)
        .fetch_one(db)
        .map_err(Error::from)
        .await
    }

    pub async fn update<'c, E>(db: E, job: &Self) -> Result<Self>
    where
        E: sqlx::Executor<'c, Database = sqlx::Postgres>,
    {
        sqlx::query_as(
            r#"
            update mailchimp set 
                mame = $2, 
                api_key = $3,
                list = $4, 
                club = $5, 
                region = $6
            where id = $1
            returning *;
            "#,
        )
        .bind(job.id)
        .bind(&job.name)
        .bind(&job.api_key)
        .bind(&job.list)
        .bind(job.club)
        .bind(job.region)
        .fetch_one(db)
        .map_err(Error::from)
        .await
    }

    pub async fn delete<'c, E>(db: E, id: i64) -> Result<()>
    where
        E: sqlx::Executor<'c, Database = sqlx::Postgres>,
    {
        sqlx::query(r#"delete from mailchimp where id = $1"#)
            .bind(id)
            .execute(db)
            .await?;
        Ok(())
    }

    fn client(&self) -> Result<mailchimp::Client> {
        Ok(mailchimp::client::from_api_key(&self.api_key)?)
    }

    async fn db_members<'c, E>(&self, db: E) -> Result<Vec<ddb::members::Member>>
    where
        E: sqlx::Executor<'c, Database = sqlx::MySql>,
    {
        let db_members = if let Some(club) = self.club {
            ddb::members::by_club(db, club as u64).await?
        } else if let Some(region) = self.region {
            ddb::members::by_region(db, region as u64).await?
        } else {
            ddb::members::all(db).await?
        };
        Ok(db_members)
    }

    fn merge_fields(&self) -> Result<mailchimp::merge_fields::MergeFields> {
        let str = if self.club.is_some() {
            include_str!("../../../data/fields-club.toml")
        } else {
            // region or all
            include_str!("../../../data/fields-all.toml")
        };

        mailchimp::merge_fields::MergeFields::from_config(config::File::from_str(
            str,
            config::FileFormat::Toml,
        ))
        .map_err(Error::from)
    }

    #[tracing::instrument(skip_all, name = "merge_fields", fields(name = self.name, id = self.id))]
    pub async fn sync_merge_fields(
        &self,
        process_deletes: bool,
    ) -> Result<(Vec<String>, Vec<String>, Vec<String>)> {
        let client = self.client()?;
        mailchimp::merge_fields::sync(&client, &self.list, self.merge_fields()?, process_deletes)
            .map_err(Error::from)
            .await
    }

    #[tracing::instrument(skip_all, name = "sync", fields(name = self.name, id = self.id))]
    pub async fn sync(&self, ddb_url: AciDatabaseSettings) -> Result {
        let db = ddb_url.connect().await?;
        let db_members = self.db_members(&db).await?;
        let merge_fields = self.merge_fields()?;

        // Fetch addresses for primary members
        tracing::info!("querying ddb");
        let db_addresses =
            ddb::members::mailing_address::for_members(&db, db_members.iter()).await?;

        // Convert ddb members to mailchimp members while injecting address
        let mc_members = ddb::members::mailchimp::to_members_with_address(
            &db_members,
            &db_addresses,
            &merge_fields,
        )
        .await?;

        let client = self.client()?;
        let upserted =
            mailchimp::members::upsert_many(&client, &self.list, futures::stream::iter(mc_members))
                .await?;

        let deleted = mailchimp::members::retain(&client, &self.list, &upserted).await?;

        let tag_updates = ddb::members::mailchimp::to_tag_updates(&db_members);
        mailchimp::members::tags::update_many(&client, &self.list, &tag_updates).await?;

        tracing::info!(deleted, upserted = upserted.len(), "completed");

        Ok(())
    }
}
