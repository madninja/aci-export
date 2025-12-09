use crate::{DB_INSERT_CHUNK_SIZE, Error, Result, retain_with_keys, user};
use futures::TryStreamExt;
use futures::{StreamExt, stream};
use sqlx::{PgPool, Postgres, QueryBuilder};

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
        u.birthday,
        u.phone_mobile,
        u.phone_home,
        u.last_login,

        r.uid as role_uid,
        r.title as role_title
    FROM
        leadership_international li
        JOIN users u ON u.id = li.user_id
        JOIN leadership_role r ON r.uid = li.role
"#;

pub async fn all(pool: &PgPool) -> Result<Vec<Leadership>> {
    sqlx::query_as::<_, Leadership>(FETCH_LEADERSHIP_QUERY)
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

    let affected: Vec<u64> = stream::iter(leadership)
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

    let mut builder = QueryBuilder::<Postgres>::new(
        "DELETE FROM leadership_international WHERE (user_id, role, start_date) NOT IN (",
    );

    for (i, lead) in leadership.iter().enumerate() {
        if i > 0 {
            builder.push(", ");
        }
        builder.push("(");
        builder.push_bind(&lead.user.id);
        builder.push(", ");
        builder.push_bind(lead.role.uid);
        builder.push(", ");
        builder.push_bind(lead.start_date);
        builder.push(")");
    }

    builder.push(")");

    let result = builder.build().execute(&mut *tx).await?;
    let total_affected = result.rows_affected();

    tx.commit().await?;
    Ok(total_affected)
}
