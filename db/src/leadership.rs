use crate::{DB_INSERT_CHUNK_SIZE, Error, Result, retain_with_keys, user};
use chrono::NaiveDate;
use futures::TryStreamExt;
use futures::{StreamExt, stream};
use itertools::Itertools;
use sqlx::{PgPool, Postgres, QueryBuilder};

/// Filter for leadership queries by date
#[derive(Debug, Clone, Default)]
pub enum DateFilter {
    /// Only currently active leadership (default)
    #[default]
    Current,
    /// All leadership regardless of dates
    All,
    /// Leadership active on a specific date
    AsOf(NaiveDate),
}

pub fn apply_date_filter(query: &mut QueryBuilder<Postgres>, filter: &DateFilter, has_where: bool) {
    let prefix = if has_where { " AND " } else { " WHERE " };
    match filter {
        DateFilter::Current => {
            query.push(prefix);
            query.push(
                "start_date <= CURRENT_DATE AND (end_date IS NULL OR end_date >= CURRENT_DATE)",
            );
        }
        DateFilter::All => {}
        DateFilter::AsOf(date) => {
            query.push(prefix);
            query.push("start_date <= ").push_bind(*date);
            query
                .push(" AND (end_date IS NULL OR end_date >= ")
                .push_bind(*date)
                .push(")");
        }
    }
}

// ========== Role Struct ==========

#[derive(Debug, sqlx::FromRow, serde::Serialize, Clone)]
pub struct Role {
    pub uid: i64,
    pub title: String,
}

// ========== International Leadership Struct ==========

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Leadership {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    #[sqlx(flatten)]
    pub user: user::User,
    #[sqlx(flatten, try_from = "RoleRef")]
    pub role: Role,
    pub start_date: chrono::NaiveDate,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<chrono::NaiveDate>,
}

// Intermediary struct for sqlx extraction
#[derive(Debug, sqlx::FromRow)]
struct RoleRef {
    role_uid: i64,
    role_title: String,
}

impl From<RoleRef> for Role {
    fn from(value: RoleRef) -> Self {
        Self {
            uid: value.role_uid,
            title: value.role_title,
        }
    }
}

// ========== Role Query Functions ==========

const FETCH_ROLES_QUERY: &str = r#"
    SELECT
        uid,
        title
    FROM
        leadership_role
"#;

pub async fn all_roles(pool: &PgPool) -> Result<Vec<Role>> {
    sqlx::query_as::<_, Role>(FETCH_ROLES_QUERY)
        .fetch_all(pool)
        .await
        .map_err(Error::from)
}

// ========== International Leadership Query Functions ==========

const FETCH_LEADERSHIP_QUERY: &str = r#"
    SELECT
        li.id,
        li.start_date,
        li.end_date,

        u.id,
        u.uid,
        u.email,
        u.first_name,
        u.last_name,

        r.uid as role_uid,
        r.title as role_title
    FROM
        leadership_international li
        JOIN users u ON u.id = li.user_id
        JOIN leadership_role r ON r.uid = li.role
"#;

pub async fn all(pool: &PgPool, filter: DateFilter) -> Result<Vec<Leadership>> {
    let mut query = QueryBuilder::new(FETCH_LEADERSHIP_QUERY);
    apply_date_filter(&mut query, &filter, false);
    query
        .build_query_as::<Leadership>()
        .fetch_all(pool)
        .await
        .map_err(Error::from)
}

// ========== Role Upsert/Retain Functions ==========

pub async fn upsert_roles(pool: &PgPool, roles: &[Role]) -> Result<u64> {
    if roles.is_empty() {
        return Ok(0);
    }
    let result = QueryBuilder::new("INSERT INTO leadership_role(uid, title) ")
        .push_values(roles, |mut b, role| {
            b.push_bind(role.uid).push_bind(&role.title);
        })
        .push(
            r#"ON CONFLICT(uid) DO UPDATE SET
                title = excluded.title
            "#,
        )
        .build()
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn retain_roles(pool: &PgPool, roles: &[Role]) -> Result<u64> {
    retain_with_keys(pool, "leadership_role", "uid", roles, |role| role.uid).await
}

// ========== International Leadership Upsert/Retain Functions ==========

pub async fn upsert_leadership(pool: &PgPool, leadership: &[Leadership]) -> Result<u64> {
    if leadership.is_empty() {
        return Ok(0);
    }

    let affected: Vec<u64> = stream::iter(
        leadership
            .iter()
            .unique_by(|l| (&l.user.id, l.role.uid, l.start_date)),
    )
    .chunks(DB_INSERT_CHUNK_SIZE)
    .map(Ok::<_, Error>)
    .and_then(|chunk| async move {
        let result = QueryBuilder::new(
            r#"INSERT INTO leadership_international(
                    user_id,
                    role,
                    start_date,
                    end_date
                ) "#,
        )
        .push_values(&chunk, |mut b, lead| {
            b.push_bind(&lead.user.id)
                .push_bind(lead.role.uid)
                .push_bind(lead.start_date)
                .push_bind(lead.end_date);
        })
        .push(
            r#"ON CONFLICT(user_id, role, start_date) DO UPDATE SET
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
        r#"CREATE TEMP TABLE _keep_leadership_international (
            user_id TEXT,
            role BIGINT,
            start_date DATE
        ) ON COMMIT DROP"#,
    )
    .execute(&mut *tx)
    .await?;

    // Insert keys in chunks
    for chunk in leadership.chunks(DB_INSERT_CHUNK_SIZE) {
        QueryBuilder::new("INSERT INTO _keep_leadership_international(user_id, role, start_date) ")
            .push_values(chunk, |mut b, lead| {
                b.push_bind(&lead.user.id)
                    .push_bind(lead.role.uid)
                    .push_bind(lead.start_date);
            })
            .build()
            .execute(&mut *tx)
            .await?;
    }

    // Delete rows not in temp table
    let result = sqlx::query(
        r#"DELETE FROM leadership_international li
           WHERE NOT EXISTS (
               SELECT 1 FROM _keep_leadership_international k
               WHERE k.user_id = li.user_id
                 AND k.role = li.role
                 AND k.start_date = li.start_date
           )"#,
    )
    .execute(&mut *tx)
    .await?;

    let total_affected = result.rows_affected();
    tx.commit().await?;
    Ok(total_affected)
}
