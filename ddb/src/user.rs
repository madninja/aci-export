use crate::Result;
use sqlx::mysql::{MySql, MySqlExecutor};

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct User {
    pub uid: u64,
    pub email: String,
    pub first_name: String,
    pub last_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birthday: Option<chrono::NaiveDate>,
}

impl User {
    fn fetch_one_query<'builder>() -> sqlx::QueryBuilder<'builder, MySql> {
        sqlx::QueryBuilder::new(
            r#"
            SELECT DISTINCT
                users_field_data.uid AS uid,
                users_field_data.mail as email,
                user__field_first_name.field_first_name_value AS first_name,
                user__field_last_name.field_last_name_value AS last_name,
                CAST(user__field_birth_date.field_birth_date_value AS DATE) AS birthday
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

    pub async fn by_uid(exec: impl MySqlExecutor<'_>, uid: u64) -> Result<Option<Self>> {
        let user = Self::fetch_one_query()
            .push("users_field_data.uid = ")
            .push_bind(uid)
            .build_query_as::<Self>()
            .fetch_optional(exec)
            .await?;

        Ok(user)
    }

    pub async fn by_email(exec: impl MySqlExecutor<'_>, email: &str) -> Result<Option<Self>> {
        let user = Self::fetch_one_query()
            .push("users_field_data.mail = ")
            .push_bind(email)
            .build_query_as::<Self>()
            .fetch_optional(exec)
            .await?;

        Ok(user)
    }
}
