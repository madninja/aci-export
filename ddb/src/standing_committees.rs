use crate::{Error, Result};
use futures::TryFutureExt;
use sqlx::{MySql, MySqlPool};

pub async fn all(pool: &MySqlPool) -> Result<Vec<StandingCommittee>> {
    sqlx::query_as::<_, StandingCommittee>(FETCH_STANDING_COMMITTEES_QUERY)
        .fetch_all(pool)
        .map_err(Error::from)
        .await
}

pub async fn by_uid(pool: &MySqlPool, uid: u64) -> Result<Option<StandingCommittee>> {
    fetch_standing_committees_query()
        .push(" WHERE nd.nid = ")
        .push_bind(uid)
        .build_query_as::<StandingCommittee>()
        .fetch_optional(pool)
        .map_err(Error::from)
        .await
}

const FETCH_STANDING_COMMITTEES_QUERY: &str = r#"
    SELECT
        nd.nid AS uid,
        nd.title AS name,
        nd.status AS active
    FROM node_field_data nd
    WHERE nd.type = 'ssp_standing_committees'
"#;

fn fetch_standing_committees_query<'builder>() -> sqlx::QueryBuilder<'builder, MySql> {
    sqlx::QueryBuilder::new(FETCH_STANDING_COMMITTEES_QUERY)
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct StandingCommittee {
    pub uid: u64,
    pub name: String,
    pub active: bool,
}

pub mod db {
    use super::*;
    use ::db as app_db;

    impl From<StandingCommittee> for app_db::standing_committee::StandingCommittee {
        fn from(value: StandingCommittee) -> Self {
            Self {
                uid: value.uid as i64,
                name: value.name,
                active: value.active,
            }
        }
    }

    impl From<crate::leadership::Leadership> for app_db::standing_committee::Leadership {
        fn from(value: crate::leadership::Leadership) -> Self {
            Self {
                id: None,
                standing_committee: app_db::standing_committee::StandingCommittee {
                    uid: value.entity_uid as i64,
                    name: String::new(),
                    active: true,
                },
                user: value.user.into(),
                role: value.role.into(),
                start_date: value.start_date,
                end_date: value.end_date,
            }
        }
    }
}
