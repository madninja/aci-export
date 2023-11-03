use crate::{clubs::Club, users::User, Error, Result, Stream};
use futures::{StreamExt, TryStreamExt};
use sqlx::{MySql, MySqlPool};

pub fn all(exec: &MySqlPool) -> Stream<Member> {
    sqlx::query_as::<_, Member>(FETCH_MEMBERS_QUERY)
        .fetch(exec)
        .map_err(Error::from)
        .boxed()
}

pub async fn by_uid(exec: &MySqlPool, uid: u64) -> Result<Option<Member>> {
    let member = fetch_members_query()
        .push("AND users_field_data.uid = ")
        .push_bind(uid)
        .build_query_as::<Member>()
        .fetch_optional(exec)
        .await?;

    Ok(member)
}

pub async fn by_email(exec: &MySqlPool, email: &str) -> Result<Option<Member>> {
    let member = fetch_members_query()
        .push("AND users_field_data.mail = ")
        .push_bind(email)
        .build_query_as::<Member>()
        .fetch_optional(exec)
        .await?;

    Ok(member)
}

const FETCH_MEMBERS_QUERY: &str = r#"
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
        CAST(partner__field_birth_date.field_birth_date_value AS DATE) AS partner_birthday,

        MembershipExpireYear AS expiration_date,
        MembershipJoinYear AS join_date,

        club_data.name AS club_name,
        club_data.uid AS club_uid,
        club_data.number AS club_number,
        club_data.region AS club_region

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
        LEFT JOIN (
            SELECT
                home_club_membership.user_id AS user_id,
                fd.title AS name,
                fd.nid AS uid,
                nc.field_club_number_value as number,
                rn.field_region_number_value as region
            FROM
                ssp_membership_home_club home_club_membership
                INNER JOIN paragraph__field_club ed ON home_club_membership.paragraph_id = ed.entity_id
                INNER JOIN node_field_data fd ON ed.field_club_target_id = fd.nid
                LEFT JOIN node__field_club_number nc ON fd.nid = nc.entity_id
                LEFT JOIN node__field_region nr ON nr.entity_id = nc.entity_id
                LEFT JOIN node__field_region_number rn ON rn.entity_id = nr.field_region_target_id
        ) club_data ON users_field_data.uid = club_data.user_id        
        INNER JOIN user__field_personal_status ON users_field_data.uid = user__field_personal_status.entity_id 
            AND user__field_personal_status.field_personal_status_target_id = 947 
        INNER JOIN user__roles ON users_field_data.uid = user__roles.entity_id
        INNER JOIN (
            SELECT
                n.entity_id AS entity_id,
                MAX(CAST(ld.field_leave_date_value AS DATE)) AS MembershipExpireYear,
                MIN(CAST(jd.field_join_date_value AS DATE)) AS MembershipJoinYear
            FROM
                user__field_international_membership n
                INNER JOIN paragraphs_item_field_data fd ON fd.revision_id = n.field_international_membership_target_revision_id
                INNER JOIN paragraph__field_join_date jd ON fd.id = jd.entity_id
                INNER JOIN paragraph__field_leave_date ld ON fd.id = ld.entity_id
                INNER JOIN paragraph__field_type ft ON fd.id = ft.entity_id
            GROUP BY
                n.entity_id
            ORDER BY
                MembershipExpireYear DESC
        ) int_membership ON users_field_data.uid = int_membership.entity_id
    
    WHERE
        users_field_data.mail IS NOT NULL
        AND user__field_primary_member.field_primary_member_target_id IS NOT NULL
    "#;

fn fetch_members_query<'builder>() -> sqlx::QueryBuilder<'builder, MySql> {
    sqlx::QueryBuilder::new(FETCH_MEMBERS_QUERY)
}

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

pub async fn mailing_address_by_uid(exec: &MySqlPool, uid: u64) -> Result<Option<Address>> {
    let member = fetch_mailing_address_query()
        .push("user__field_address.entity_id = ")
        .push_bind(uid)
        .build_query_as::<Address>()
        .fetch_optional(exec)
        .await?;
    Ok(member)
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Member {
    #[sqlx(flatten)]
    pub primary: User,
    #[sqlx(flatten, try_from = "PartnerUser")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partner: Option<User>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<chrono::NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join_date: Option<chrono::NaiveDate>,
    #[sqlx(flatten, try_from = "LocalClub")]
    pub local_club: Club,
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Address {
    pub street_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub street_address_2: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zip_code: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<String>,
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
                first_name: value.partner_first_name,
                last_name: value.partner_last_name,
                birthday: value.partner_birthday,
            })
        } else {
            None
        }
    }
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
struct LocalClub {
    club_name: Option<String>,
    club_uid: Option<u64>,
    club_number: Option<i64>,
    club_region: Option<i64>,
}

impl From<LocalClub> for Club {
    fn from(value: LocalClub) -> Club {
        Club {
            uid: value.club_uid.unwrap_or_default(),
            number: value.club_number,
            name: value.club_name.unwrap_or_default(),
            region: value.club_region.unwrap_or_default(),
        }
    }
}
