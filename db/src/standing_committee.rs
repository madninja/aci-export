use crate::{DB_INSERT_CHUNK_SIZE, Error, Result, leadership, retain_with_keys, user};
use futures::{StreamExt, TryFutureExt, TryStreamExt, stream};
use itertools::Itertools;
use sqlx::{PgPool, Postgres, QueryBuilder};

pub async fn all(pool: &PgPool) -> Result<Vec<StandingCommittee>> {
    sqlx::query_as::<_, StandingCommittee>(FETCH_STANDING_COMMITTEES_QUERY)
        .fetch_all(pool)
        .map_err(Error::from)
        .await
}

pub async fn by_uid(pool: &PgPool, uid: i64) -> Result<Option<StandingCommittee>> {
    fetch_standing_committees_query()
        .push(" WHERE uid = ")
        .push_bind(uid)
        .build_query_as::<StandingCommittee>()
        .fetch_optional(pool)
        .map_err(Error::from)
        .await
}

pub async fn upsert_many(pool: &PgPool, committees: &[StandingCommittee]) -> Result<u64> {
    if committees.is_empty() {
        return Ok(0);
    }

    let mut total_affected = 0u64;
    for chunk in committees.chunks(DB_INSERT_CHUNK_SIZE) {
        let result = QueryBuilder::new("INSERT INTO standing_committees(uid, name, active) ")
            .push_values(chunk, |mut b, committee| {
                b.push_bind(committee.uid)
                    .push_bind(&committee.name)
                    .push_bind(committee.active);
            })
            .push(
                r#"ON CONFLICT(uid) DO UPDATE SET
                    name = excluded.name,
                    active = excluded.active
                "#,
            )
            .build()
            .execute(pool)
            .await?;
        total_affected += result.rows_affected();
    }
    Ok(total_affected)
}

pub async fn retain(pool: &PgPool, committees: &[StandingCommittee]) -> Result<u64> {
    retain_with_keys(pool, "standing_committees", "uid", committees, |c| c.uid).await
}

const FETCH_STANDING_COMMITTEES_QUERY: &str = r#"
    SELECT
        uid,
        name,
        active
    FROM standing_committees
"#;

fn fetch_standing_committees_query<'builder>() -> QueryBuilder<'builder, Postgres> {
    QueryBuilder::new(FETCH_STANDING_COMMITTEES_QUERY)
}

#[derive(Debug, sqlx::FromRow, serde::Serialize, Clone)]
pub struct StandingCommittee {
    pub uid: i64,
    pub name: String,
    pub active: bool,
}

// ========== Standing Committee Leadership ==========

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Leadership {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[sqlx(flatten, try_from = "StandingCommitteeRef")]
    pub standing_committee: StandingCommittee,
    #[sqlx(flatten)]
    pub user: user::User,
    #[sqlx(flatten, try_from = "RoleRef")]
    pub role: leadership::Role,
    pub start_date: chrono::NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<chrono::NaiveDate>,
}

// Intermediary structs for sqlx extraction
#[derive(Debug, sqlx::FromRow)]
struct StandingCommitteeRef {
    standing_committee_uid: i64,
    standing_committee_name: String,
    standing_committee_active: bool,
}

impl From<StandingCommitteeRef> for StandingCommittee {
    fn from(value: StandingCommitteeRef) -> Self {
        Self {
            uid: value.standing_committee_uid,
            name: value.standing_committee_name,
            active: value.standing_committee_active,
        }
    }
}

#[derive(Debug, sqlx::FromRow)]
struct RoleRef {
    role_uid: i64,
    role_title: String,
}

impl From<RoleRef> for leadership::Role {
    fn from(value: RoleRef) -> Self {
        Self {
            uid: value.role_uid,
            title: value.role_title,
        }
    }
}

const FETCH_LEADERSHIP_QUERY: &str = r#"
    SELECT
        lsc.id,
        lsc.start_date,
        lsc.end_date,

        sc.uid as standing_committee_uid,
        sc.name as standing_committee_name,
        sc.active as standing_committee_active,

        u.id,
        u.uid,
        u.email,
        u.first_name,
        u.last_name,
        u.birthday,
        u.phone_mobile,
        u.phone_home,
        u.last_login,

        r.uid as role_uid,
        r.title as role_title
    FROM
        leadership_standing_committee lsc
        JOIN standing_committees sc ON sc.uid = lsc.standing_committee
        JOIN users u ON u.id = lsc.user_id
        JOIN leadership_role r ON r.uid = lsc.role
"#;

fn fetch_leadership_query<'builder>() -> QueryBuilder<'builder, Postgres> {
    QueryBuilder::new(FETCH_LEADERSHIP_QUERY)
}

pub async fn all_leadership(
    pool: &PgPool,
    filter: leadership::DateFilter,
) -> Result<Vec<Leadership>> {
    let mut query = fetch_leadership_query();
    leadership::apply_date_filter(&mut query, &filter, false);
    query
        .build_query_as::<Leadership>()
        .fetch_all(pool)
        .await
        .map_err(Error::from)
}

pub async fn leadership_by_uid(
    pool: &PgPool,
    committee_uid: i64,
    filter: leadership::DateFilter,
) -> Result<Vec<Leadership>> {
    let mut query = fetch_leadership_query();
    query.push("WHERE sc.uid = ").push_bind(committee_uid);
    leadership::apply_date_filter(&mut query, &filter, true);
    query
        .build_query_as::<Leadership>()
        .fetch_all(pool)
        .await
        .map_err(Error::from)
}

pub async fn upsert_leadership(pool: &PgPool, leadership: &[Leadership]) -> Result<u64> {
    if leadership.is_empty() {
        return Ok(0);
    }

    let affected: Vec<u64> = stream::iter(leadership.iter().unique_by(|l| {
        (
            l.standing_committee.uid,
            &l.user.id,
            l.role.uid,
            l.start_date,
        )
    }))
    .chunks(DB_INSERT_CHUNK_SIZE)
    .map(Ok::<_, Error>)
    .and_then(|chunk| async move {
        let result = QueryBuilder::new(
            r#"INSERT INTO leadership_standing_committee(
                    standing_committee,
                    user_id,
                    role,
                    start_date,
                    end_date
                ) "#,
        )
        .push_values(&chunk, |mut b, lead| {
            b.push_bind(lead.standing_committee.uid)
                .push_bind(&lead.user.id)
                .push_bind(lead.role.uid)
                .push_bind(lead.start_date)
                .push_bind(lead.end_date);
        })
        .push(
            r#"ON CONFLICT(standing_committee, user_id, role, start_date) DO UPDATE SET
                    end_date = excluded.end_date
                "#,
        )
        .build()
        .execute(pool)
        .await?;
        Ok::<u64, Error>(result.rows_affected())
    })
    .try_collect()
    .await?;
    Ok(affected.iter().sum())
}

pub async fn retain_leadership(pool: &PgPool, leadership: &[Leadership]) -> Result<u64> {
    if leadership.is_empty() {
        return Ok(0);
    }

    let mut tx = pool.begin().await?;

    // Create temp table to hold keys to keep
    sqlx::query(
        r#"CREATE TEMP TABLE _keep_leadership_standing_committee (
            standing_committee BIGINT,
            user_id TEXT,
            role BIGINT,
            start_date DATE
        ) ON COMMIT DROP"#,
    )
    .execute(&mut *tx)
    .await?;

    // Insert keys in chunks
    for chunk in leadership.chunks(DB_INSERT_CHUNK_SIZE) {
        QueryBuilder::new(
            "INSERT INTO _keep_leadership_standing_committee(standing_committee, user_id, role, start_date) ",
        )
        .push_values(chunk, |mut b, lead| {
            b.push_bind(lead.standing_committee.uid)
                .push_bind(&lead.user.id)
                .push_bind(lead.role.uid)
                .push_bind(lead.start_date);
        })
        .build()
        .execute(&mut *tx)
        .await?;
    }

    // Delete rows not in temp table
    let result = sqlx::query(
        r#"DELETE FROM leadership_standing_committee lsc
           WHERE NOT EXISTS (
               SELECT 1 FROM _keep_leadership_standing_committee k
               WHERE k.standing_committee = lsc.standing_committee
                 AND k.user_id = lsc.user_id
                 AND k.role = lsc.role
                 AND k.start_date = lsc.start_date
           )"#,
    )
    .execute(&mut *tx)
    .await?;

    let total_affected = result.rows_affected();
    tx.commit().await?;
    Ok(total_affected)
}
