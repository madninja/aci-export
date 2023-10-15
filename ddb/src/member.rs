use crate::{user::User, Result};
use sqlx::mysql::{MySql, MySqlExecutor};

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Member {
    #[sqlx(flatten)]
    pub primary: User,
    #[sqlx(flatten, try_from = "PartnerUser")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partner: Option<User>,
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Address {
    street_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    street_address_2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    zip_code: Option<String>,
    city: String,
    state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    country: Option<String>,
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
struct PartnerUser {
    partner_uid: Option<u64>,
    partner_email: Option<String>,
    partner_first_name: Option<String>,
    partner_last_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    partner_birthday: Option<chrono::NaiveDate>,
}

impl From<PartnerUser> for Option<User> {
    fn from(value: PartnerUser) -> Option<User> {
        if let Some(uid) = value.partner_uid {
            Some(User {
                uid,
                email: value.partner_email.unwrap(),
                first_name: value.partner_first_name.unwrap(),
                last_name: value.partner_last_name.unwrap(),
                birthday: value.partner_birthday,
            })
        } else {
            None
        }
    }
}

impl Member {
    fn fetch_one_query<'builder>() -> sqlx::QueryBuilder<'builder, MySql> {
        sqlx::QueryBuilder::new(
            r#"
            SELECT DISTINCT
                users_field_data.uid AS uid,
                users_field_data.mail as email,
                user__field_first_name.field_first_name_value AS first_name,
                user__field_last_name.field_last_name_value AS last_name,
                CAST(user__field_birth_date.field_birth_date_value AS DATE) AS birthday,
            
                partner_field_data.uid AS partner_uid,
                partner_field_data.mail AS partner_email,
                partner__field_first_name.field_first_name_value AS partner_first_name,
                partner__field_last_name.field_last_name_value AS partner_last_name,
                CAST(partner__field_birth_date.field_birth_date_value AS DATE) AS partner_birthday

            FROM
                users_field_data 
                LEFT JOIN user__field_primary_member ON users_field_data.uid = user__field_primary_member.field_primary_member_target_id
                LEFT JOIN user__field_first_name ON users_field_data.uid = user__field_first_name.entity_id
                LEFT JOIN user__field_last_name ON users_field_data.uid = user__field_last_name.entity_id
                LEFT JOIN user__field_birth_date ON users_field_data.uid = user__field_birth_date.entity_id
                
                LEFT JOIN users_field_data partner_field_data ON user__field_primary_member.entity_id = partner_field_data.uid
                LEFT JOIN user__field_first_name partner__field_first_name ON partner_field_data.uid = partner__field_first_name.entity_id
                LEFT JOIN user__field_last_name partner__field_last_name ON partner_field_data.uid = partner__field_last_name.entity_id
                LEFT JOIN user__field_birth_date partner__field_birth_date ON partner_field_data.uid = partner__field_birth_date.entity_id

            WHERE
                users_field_data.mail IS NOT NULL AND
            "#,
        )
    }

    pub async fn by_uid(exec: impl MySqlExecutor<'_>, uid: u64) -> Result<Option<Self>> {
        let member = Self::fetch_one_query()
            .push("users_field_data.uid = ")
            .push_bind(uid)
            .build_query_as::<Self>()
            .fetch_optional(exec)
            .await?;

        Ok(member)
    }

    pub async fn by_email(exec: impl MySqlExecutor<'_>, email: &str) -> Result<Option<Self>> {
        let member = Self::fetch_one_query()
            .push("users_field_data.mail = ")
            .push_bind(email)
            .build_query_as::<Self>()
            .fetch_optional(exec)
            .await?;

        Ok(member)
    }

    pub async fn mailing_address(&self, exec: impl MySqlExecutor<'_>) -> Result<Option<Address>> {
        Address::mailing_address_by_uid(exec, self.primary.uid).await
    }

    pub async fn expiration_date(
        &self,
        exec: impl MySqlExecutor<'_>,
    ) -> Result<Option<sqlx::types::chrono::NaiveDate>> {
        Self::expiration_date_by_uid(exec, self.primary.uid).await
    }

    pub async fn expiration_date_by_uid(
        exec: impl MySqlExecutor<'_>,
        uid: u64,
    ) -> Result<Option<sqlx::types::chrono::NaiveDate>> {
        let date = sqlx::query_scalar::<_, chrono::NaiveDate>(
            r#"
            SELECT 
            	MAX(CAST(ld.field_leave_date_value AS DATE))
            FROM
             	user__field_international_membership n
                INNER JOIN paragraphs_item_field_data fd ON fd.revision_id = n.field_international_membership_target_revision_id 
                INNER JOIN paragraph__field_leave_date ld ON fd.id = ld.entity_id
            WHERE 
                n.entity_id = ?
            "#,
        )
        .bind(uid)
        .fetch_optional(exec)
        .await?;
        Ok(date)
    }

    pub async fn join_date_by_uid(
        exec: impl MySqlExecutor<'_>,
        uid: u64,
    ) -> Result<Option<sqlx::types::chrono::NaiveDate>> {
        let date = sqlx::query_scalar::<_, chrono::NaiveDate>(
            r#"
            SELECT 
                MIN(CAST(jd.field_join_date_value AS DATE))
            FROM
             	user__field_international_membership n
                INNER JOIN paragraphs_item_field_data fd ON fd.revision_id = n.field_international_membership_target_revision_id 
                INNER JOIN paragraph__field_join_date jd ON fd.id = jd.entity_id
            WHERE 
                n.entity_id = ?
            "#,
        )
        .bind(uid)
        .fetch_optional(exec)
        .await?;
        Ok(date)
    }
}

impl Address {
    fn fetch_mailing_address_query<'builder>() -> sqlx::QueryBuilder<'builder, MySql> {
        sqlx::QueryBuilder::new(
            r#"
            SELECT
                user__field_address.entity_id,
                paragraph__field_address.field_address_value AS street_address,
                paragraph__field_street_address_2.field_street_address_2_value AS street_address_2,
                paragraph__field_zip_code.field_zip_code_value AS zip_code,
                paragraph__field_city.field_city_value AS city,
                paragraph__field_state_name.field_state_name_value AS state,
                paragraph__field_country.field_country_value AS country
            FROM
                paragraph__field_use_as_mailing_address mail
                INNER JOIN user__field_address ON user__field_address.field_address_target_id = mail.entity_id
                INNER JOIN paragraph__field_address ON mail.entity_id = paragraph__field_address.entity_id
                LEFT JOIN paragraph__field_street_address_2 ON mail.entity_id = paragraph__field_street_address_2.entity_id
                LEFT JOIN paragraph__field_zip_code ON mail.entity_id = paragraph__field_zip_code.entity_id
                LEFT JOIN paragraph__field_city ON mail.entity_id = paragraph__field_city.entity_id
                LEFT JOIN paragraph__field_state_name ON mail.entity_id = paragraph__field_state_name.entity_id
                LEFT JOIN paragraph__field_country ON mail.entity_id = paragraph__field_country.entity_id
            WHERE
                mail.field_use_as_mailing_address_value = 1 AND
            "#,
        )
    }

    pub async fn mailing_address_by_uid(
        exec: impl MySqlExecutor<'_>,
        uid: u64,
    ) -> Result<Option<Self>> {
        let member = Self::fetch_mailing_address_query()
            .push("user__field_address.entity_id = ")
            .push_bind(uid)
            .build_query_as::<Self>()
            .fetch_optional(exec)
            .await?;

        Ok(member)
    }
}
