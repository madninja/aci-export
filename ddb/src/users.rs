use crate::Result;
use sqlx::{MySqlPool, mysql::MySql};

#[derive(Debug, sqlx::FromRow, serde::Serialize, Clone)]
pub struct User {
    pub uid: u64,
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birthday: Option<chrono::NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_login: Option<chrono::NaiveDate>,
}

fn fetch_user_query<'builder>() -> sqlx::QueryBuilder<'builder, MySql> {
    sqlx::QueryBuilder::new(
        r#"
            SELECT DISTINCT
                users_field_data.uid AS uid,
                users_field_data.mail as email,
                user__field_first_name.field_first_name_value AS first_name,
                user__field_last_name.field_last_name_value AS last_name,
                CAST(user__field_birth_date.field_birth_date_value AS DATE) AS birthday,
                DATE(FROM_UNIXTIME(users_field_data.login)) AS last_login
            FROM
                users_field_data
                LEFT JOIN user__field_first_name ON users_field_data.uid = user__field_first_name.entity_id
                LEFT JOIN user__field_last_name ON users_field_data.uid = user__field_last_name.entity_id
                LEFT JOIN user__field_birth_date ON users_field_data.uid = user__field_birth_date.entity_id
            WHERE
                users_field_data.mail IS NOT NULL
                AND
            "#,
    )
}

pub async fn by_uid(pool: &MySqlPool, uid: u64) -> Result<Option<User>> {
    let user = fetch_user_query()
        .push("users_field_data.uid = ")
        .push_bind(uid)
        .build_query_as::<User>()
        .fetch_optional(pool)
        .await?;

    Ok(user)
}

pub async fn by_email(pool: &MySqlPool, email: &str) -> Result<Option<User>> {
    let user = fetch_user_query()
        .push("users_field_data.mail = ")
        .push_bind(email)
        .build_query_as::<User>()
        .fetch_optional(pool)
        .await?;

    Ok(user)
}

pub mod db {
    use super::*;
    use ::db as app_db;

    impl From<User> for app_db::user::User {
        fn from(value: User) -> Self {
            Self {
                id: app_db::user::id_for_email(&value.email),
                uid: value.uid as i64,
                email: value.email,
                first_name: value.first_name,
                last_name: value.last_name,
                birthday: value.birthday,
                phone_mobile: None,
                phone_home: None,
                last_login: value.last_login,
            }
        }
    }
}
