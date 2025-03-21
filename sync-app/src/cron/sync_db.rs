use crate::{address, brn, club, member, region, settings::Settings, user, Error, Result};
use futures::TryFutureExt;
use itertools::Itertools;
use serde::Serialize;
use sqlx::PgExecutor;
use std::{collections::HashMap, time::Instant};
use tokio_cron_scheduler::{Job, JobScheduler};

pub async fn schedule(settings: Settings, scheduler: &mut JobScheduler) -> Result {
    let job = Job::new_async("@daily", {
        let inner_settings = settings.clone();
        move |_uuid, _lock| {
            Box::pin({
                let settings = inner_settings.clone();
                async move {
                    if let Err(err) = run(settings).await {
                        tracing::error!(?err, "failed to sync db");
                    }
                }
            })
        }
    })?;
    scheduler.add(job).await?;
    Ok(())
}

#[derive(Debug, Serialize)]
pub struct SyncStats {
    pub upserted: u64,
    pub deleted: u64,
    pub duration: u64,
}

impl SyncStats {
    fn new(upserted: u64, deleted: u64, duration: u64) -> Self {
        Self {
            upserted,
            deleted,
            duration,
        }
    }
}

pub type SyncStatsMap = std::collections::HashMap<String, SyncStats>;

pub async fn sync_regions<'c, DB, I>(db: DB, regions: I) -> Result<(String, SyncStats)>
where
    DB: PgExecutor<'c> + Copy,
    I: IntoIterator<Item = ddb::regions::Region>,
{
    let start = Instant::now();
    let db_regions = regions.into_iter().map(region::Region::from).collect_vec();
    let upserted = region::upsert_many(db, &db_regions).await?;
    let deleted = region::retain(db, &db_regions).await?;
    let duration = start.elapsed().as_secs();
    tracing::info!(deleted, upserted, duration, "regions completed");
    Ok((
        "regions".to_string(),
        SyncStats::new(upserted, deleted, duration),
    ))
}

pub async fn sync_clubs<'c, DB, I>(db: DB, clubs: I) -> Result<(String, SyncStats)>
where
    DB: PgExecutor<'c> + Copy,
    I: IntoIterator<Item = ddb::clubs::Club>,
{
    let start = Instant::now();
    let db_clubs = clubs.into_iter().map(club::Club::from).collect_vec();
    let upserted = club::upsert_many(db, &db_clubs).await?;
    let deleted = club::retain(db, &db_clubs).await?;
    let duration = start.elapsed().as_secs();
    tracing::info!(deleted, upserted, duration, "clubs completed");
    Ok(("clubs".into(), SyncStats::new(upserted, deleted, duration)))
}

pub async fn sync_users<'c, DB, I>(db: DB, users: I) -> Result<(String, SyncStats)>
where
    DB: PgExecutor<'c> + Copy,
    I: IntoIterator<Item = ddb::users::User>,
{
    let start = Instant::now();
    let db_users = users.into_iter().map(user::User::from).collect_vec();
    let upserted = user::upsert_many(db, &db_users).await?;
    let deleted = user::retain(db, &db_users).await?;
    let duration = start.elapsed().as_secs();
    tracing::info!(deleted, upserted, duration, "users completed");
    Ok(("users".into(), SyncStats::new(upserted, deleted, duration)))
}

pub async fn sync_members<'c, DB, I>(db: DB, members: I) -> Result<(String, SyncStats)>
where
    DB: PgExecutor<'c> + Copy,
    I: IntoIterator<Item = ddb::members::Member>,
{
    let start = Instant::now();
    let db_members = members.into_iter().map(member::Member::from).collect_vec();
    let upserted = member::upsert_many(db, &db_members).await?;
    let deleted = member::retain(db, &db_members).await?;
    let duration = start.elapsed().as_secs();
    tracing::info!(deleted, upserted, duration, "members completed");
    Ok((
        "members".into(),
        SyncStats::new(upserted, deleted, duration),
    ))
}

pub async fn sync_addresses<'c, DB>(
    db: DB,
    ddb_members: &[ddb::members::Member],
    ddb_addresses: &mut HashMap<u64, ddb::members::Address>,
) -> Result<(String, SyncStats)>
where
    DB: PgExecutor<'c> + Copy,
{
    let start = Instant::now();
    let db_addresses = ddb_members
        .iter()
        .filter_map(|ddb_member| {
            ddb_addresses
                .remove(&ddb_member.primary.uid)
                .map(|ddb_address| address::Address::from_member(ddb_member, ddb_address))
        })
        .collect_vec();
    let upserted = address::upsert_many(db, &db_addresses).await?;
    let deleted = address::retain(db, &db_addresses).await?;
    let duration = start.elapsed().as_secs();
    tracing::info!(deleted, upserted, duration, "regions completed");
    Ok((
        "addresses".to_string(),
        SyncStats::new(upserted, deleted, duration),
    ))
}

pub async fn sync_brns<'c, DB>(db: DB, brns: &[brn::Brn]) -> Result<(String, SyncStats)>
where
    DB: PgExecutor<'c> + Copy,
{
    let start = Instant::now();
    let upserted = brn::upsert_many(db, brns).await?;
    let deleted = brn::retain(db, brns).await?;
    let duration = start.elapsed().as_secs();
    tracing::info!(deleted, upserted, duration, "regions completed");
    Ok((
        "addresses".to_string(),
        SyncStats::new(upserted, deleted, duration),
    ))
}

#[tracing::instrument(skip_all, name = "sync")]
pub async fn run(settings: Settings) -> Result<SyncStatsMap> {
    let ddb = settings.ddb.connect().await?;
    let db = settings.db.connect().await?;

    tracing::info!("starting sync");
    let start = Instant::now();
    let regions = ddb::regions::all(&ddb)
        .map_err(Error::from)
        .and_then(|ddb_regions| sync_regions(&db, ddb_regions))
        .await?;
    let clubs = ddb::clubs::all(&ddb)
        .map_err(Error::from)
        .and_then(|ddb_clubs| sync_clubs(&db, ddb_clubs))
        .await?;

    let ddb_members = ddb::members::all(&ddb).await?;
    let ddb_users = ddb_members
        .iter()
        .flat_map(|ddb_member| [Some(ddb_member.primary.clone()), ddb_member.partner.clone()])
        .flatten()
        .collect_vec();
    let db_brns = ddb_members
        .iter()
        .flat_map(brn::Brn::from_member)
        .collect_vec();
    let mut ddb_addresses = ddb::members::mailing_address::for_members(&ddb, &ddb_members).await?;
    let users = sync_users(&db, ddb_users).await?;
    let addresses = sync_addresses(&db, &ddb_members, &mut ddb_addresses).await?;
    let members = sync_members(&db, ddb_members).await?;
    let brns = sync_brns(&db, &db_brns).await?;
    let duration = start.elapsed().as_secs();
    tracing::info!(duration, "sync complete");

    let stats: SyncStatsMap = [brns, addresses, regions, clubs, users, members]
        .into_iter()
        .collect();
    Ok(stats)
}
