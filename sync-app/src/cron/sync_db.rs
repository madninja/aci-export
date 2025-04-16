use crate::{address, brn, club, member, region, settings::Settings, user, Result};
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
    fn new(upserted: u64, duration: u64) -> Self {
        Self {
            upserted,
            deleted: 0,
            duration,
        }
    }
}

pub type SyncStatsMap = std::collections::HashMap<String, SyncStats>;

pub async fn upsert_regions<'c, DB, I>(
    db: DB,
    regions: I,
) -> Result<((String, SyncStats), Vec<region::Region>)>
where
    DB: PgExecutor<'c> + Copy,
    I: IntoIterator<Item = ddb::regions::Region>,
{
    let start = Instant::now();
    let db_regions = regions.into_iter().map(region::Region::from).collect_vec();
    let upserted = region::upsert_many(db, &db_regions).await?;
    let duration = start.elapsed().as_secs();
    tracing::info!(upserted, duration, "upserted regions");
    Ok((
        ("regions".to_string(), SyncStats::new(upserted, duration)),
        db_regions,
    ))
}

pub async fn retain_regions<'c, DB>(
    db: DB,
    stats: &mut (String, SyncStats),
    db_regions: &[region::Region],
) -> Result<()>
where
    DB: PgExecutor<'c> + Copy,
{
    let start = Instant::now();
    let deleted = region::retain(db, db_regions).await?;
    let duration = start.elapsed().as_secs();
    tracing::info!(deleted, duration, "gc regions");
    stats.1.deleted = deleted;
    stats.1.duration += duration;
    Ok(())
}

pub async fn upsert_clubs<'c, DB, I>(
    db: DB,
    clubs: I,
) -> Result<((String, SyncStats), Vec<club::Club>)>
where
    DB: PgExecutor<'c> + Copy,
    I: IntoIterator<Item = ddb::clubs::Club>,
{
    let start = Instant::now();
    let db_clubs = clubs.into_iter().map(club::Club::from).collect_vec();
    let upserted = club::upsert_many(db, &db_clubs).await?;
    let duration = start.elapsed().as_secs();
    tracing::info!(upserted, duration, "upserted clubs");
    Ok((
        ("clubs".into(), SyncStats::new(upserted, duration)),
        db_clubs,
    ))
}

pub async fn retain_clubs<'c, DB>(
    db: DB,
    stats: &mut (String, SyncStats),
    db_clubs: &[club::Club],
) -> Result<()>
where
    DB: PgExecutor<'c> + Copy,
{
    let start = Instant::now();
    let deleted = club::retain(db, db_clubs).await?;
    let duration = start.elapsed().as_secs();
    tracing::info!(deleted, duration, "gc clubs");
    stats.1.deleted = deleted;
    stats.1.duration += duration;
    Ok(())
}

pub async fn upsert_users<'c, DB, I>(
    db: DB,
    users: I,
) -> Result<((String, SyncStats), Vec<user::User>)>
where
    DB: PgExecutor<'c> + Copy,
    I: IntoIterator<Item = ddb::users::User>,
{
    let start = Instant::now();
    let db_users = users.into_iter().map(user::User::from).collect_vec();
    let upserted = user::upsert_many(db, &db_users).await?;
    let duration = start.elapsed().as_secs();
    tracing::info!(upserted, duration, "upserted users");
    Ok((
        ("users".into(), SyncStats::new(upserted, duration)),
        db_users,
    ))
}

pub async fn retain_users<'c, DB>(
    db: DB,
    stats: &mut (String, SyncStats),
    db_users: &[user::User],
) -> Result<()>
where
    DB: PgExecutor<'c> + Copy,
{
    let start = Instant::now();
    let deleted = user::retain(db, db_users).await?;
    let duration = start.elapsed().as_secs();
    tracing::info!(deleted, duration, "gc users");
    stats.1.deleted = deleted;
    stats.1.duration += duration;
    Ok(())
}

pub async fn upsert_members<'c, DB, I>(
    db: DB,
    members: I,
) -> Result<((String, SyncStats), Vec<member::Member>)>
where
    DB: PgExecutor<'c> + Copy,
    I: IntoIterator<Item = ddb::members::Member>,
{
    let start = Instant::now();
    let db_members = members.into_iter().map(member::Member::from).collect_vec();
    let upserted = member::upsert_many(db, &db_members).await?;
    let duration = start.elapsed().as_secs();
    tracing::info!(upserted, duration, "upserted members");
    Ok((
        ("members".into(), SyncStats::new(upserted, duration)),
        db_members,
    ))
}

pub async fn retain_members<'c, DB>(
    db: DB,
    stats: &mut (String, SyncStats),
    db_members: &[member::Member],
) -> Result<()>
where
    DB: PgExecutor<'c> + Copy,
{
    let start = Instant::now();
    let deleted = member::retain(db, db_members).await?;
    let duration = start.elapsed().as_secs();
    tracing::info!(deleted, duration, "gc members");
    stats.1.deleted = deleted;
    stats.1.duration += duration;
    Ok(())
}

pub async fn upsert_addresses<'c, DB>(
    db: DB,
    ddb_members: &[ddb::members::Member],
    ddb_addresses: &mut HashMap<u64, ddb::members::Address>,
) -> Result<((String, SyncStats), Vec<address::Address>)>
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
    tracing::info!(deleted, upserted, duration, "upserted addresses");
    Ok((
        ("addresses".to_string(), SyncStats::new(upserted, duration)),
        db_addresses,
    ))
}

pub async fn retain_addresses<'c, DB>(
    db: DB,
    stats: &mut (String, SyncStats),
    db_addresses: &[address::Address],
) -> Result<()>
where
    DB: PgExecutor<'c> + Copy,
{
    let start = Instant::now();
    let deleted = address::retain(db, db_addresses).await?;
    let duration = start.elapsed().as_secs();
    tracing::info!(deleted, duration, "gc addresses");
    stats.1.deleted = deleted;
    stats.1.duration += duration;
    Ok(())
}

pub async fn upsert_brns<'c, DB>(
    db: DB,
    db_brns: &[brn::Brn],
) -> Result<((String, SyncStats), Vec<brn::Brn>)>
where
    DB: PgExecutor<'c> + Copy,
{
    let start = Instant::now();
    let upserted = brn::upsert_many(db, db_brns).await?;
    let duration = start.elapsed().as_secs();
    tracing::info!(upserted, duration, "upserted brns");
    Ok((
        ("brns".to_string(), SyncStats::new(upserted, duration)),
        db_brns.to_vec(),
    ))
}

pub async fn retain_brns<'c, DB>(
    db: DB,
    stats: &mut (String, SyncStats),
    db_brns: &[brn::Brn],
) -> Result<()>
where
    DB: PgExecutor<'c> + Copy,
{
    let start = Instant::now();
    let deleted = brn::retain(db, db_brns).await?;
    let duration = start.elapsed().as_secs();
    tracing::info!(deleted, duration, "gc brns");
    stats.1.deleted = deleted;
    stats.1.duration += duration;
    Ok(())
}

#[tracing::instrument(skip_all, name = "sync")]
pub async fn run(settings: Settings) -> Result<SyncStatsMap> {
    let ddb = settings.ddb.connect().await?;
    let db = settings.db.connect().await?;

    tracing::info!("starting sync");
    let start = Instant::now();

    let ddb_regions = ddb::regions::all(&ddb).await?;
    let ddb_clubs = ddb::clubs::all(&ddb).await?;
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

    let (mut region_stats, db_regions) = upsert_regions(&db, ddb_regions).await?;
    let (mut club_stats, db_clubs) = upsert_clubs(&db, ddb_clubs).await?;
    let (mut user_stats, db_users) = upsert_users(&db, ddb_users).await?;
    let (mut address_stats, db_addresses) =
        upsert_addresses(&db, &ddb_members, &mut ddb_addresses).await?;
    let (mut member_stats, db_members) = upsert_members(&db, ddb_members).await?;
    let (mut brn_stats, db_brns) = upsert_brns(&db, &db_brns).await?;

    retain_clubs(&db, &mut club_stats, &db_clubs).await?;
    retain_regions(&db, &mut region_stats, &db_regions).await?;
    retain_brns(&db, &mut brn_stats, &db_brns).await?;
    retain_members(&db, &mut member_stats, &db_members).await?;

    retain_addresses(&db, &mut address_stats, &db_addresses).await?;
    retain_users(&db, &mut user_stats, &db_users).await?;

    let duration = start.elapsed().as_secs();
    tracing::info!(duration, "sync complete");

    let stats: SyncStatsMap = [
        brn_stats,
        address_stats,
        region_stats,
        club_stats,
        user_stats,
        member_stats,
    ]
    .into_iter()
    .collect();
    Ok(stats)
}
