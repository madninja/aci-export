use crate::Result;
use sqlx::mysql::{MySql, MySqlPool};

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
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
                CAST(user__field_birth_date.field_birth_date_value AS DATE) AS birthday
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

pub async fn by_uid(exec: &MySqlPool, uid: u64) -> Result<Option<User>> {
    let user = fetch_user_query()
        .push("users_field_data.uid = ")
        .push_bind(uid)
        .build_query_as::<User>()
        .fetch_optional(exec)
        .await?;

    Ok(user)
}

pub async fn by_email(exec: &MySqlPool, email: &str) -> Result<Option<User>> {
    let user = fetch_user_query()
        .push("users_field_data.mail = ")
        .push_bind(email)
        .build_query_as::<User>()
        .fetch_optional(exec)
        .await?;

    Ok(user)
}
