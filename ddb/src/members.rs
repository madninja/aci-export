use crate::{clubs::Club, users::User, Error, Result, Stream};
use constcat::concat;
use futures::{StreamExt, TryStreamExt};
use sqlx::{MySql, MySqlPool};

pub fn all(exec: &MySqlPool) -> Stream<Member> {
    const QUERY: &str = concat!(
        FETCH_MEMBERS_QUERY,
        " AND paragraphs_item_field_data.parent_field_name = 'field_home_club'"
    );
    sqlx::query_as::<_, Member>(QUERY)
        .fetch(exec)
        .map_err(Error::from)
        .boxed()
}

pub async fn by_club(exec: &MySqlPool, uid: u64) -> Result<Vec<Member>> {
    let members = fetch_members_query()
        .push("AND node_field_data_paragraph__field_club.nid = ")
        .push_bind(uid)
        .build_query_as::<Member>()
        .fetch_all(exec)
        .await?;
    Ok(members)
}

pub async fn by_region(exec: &MySqlPool, uid: u64) -> Result<Vec<Member>> {
    let members = fetch_members_query()
        .push("AND node_field_data_node__field_region.nid = ")
        .push_bind(uid)
        .build_query_as::<Member>()
        .fetch_all(exec)
        .await?;
    Ok(members)
}

pub async fn by_uid(exec: &MySqlPool, uid: u64) -> Result<Option<Member>> {
    let member = fetch_members_query()
        .push("AND paragraphs_item_field_data.parent_field_name = 'field_home_club'")
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
    SELECT
    	users_field_data.uid AS uid,
    	alldata.email AS email,
    	alldata.first_name AS first_name,
    	alldata.last_name AS last_name,
        CAST(alldata.birthdate AS DATE) AS birthday,

       	CAST(alldata.partner_user_id AS UNSIGNED) AS partner_uid,
    	alldata.partner_email AS partner_email,
    	alldata.partner_first_name AS partner_first_name,
    	alldata.partner_last_name AS partner_last_name,
        CAST(alldata.partner_birthdate AS DATE) AS partner_birthday,

    	IF(memclassterm.name IS NULL, "Regular", memclassterm.name) AS member_class,
    	paragraphs_item_field_data.parent_field_name AS member_type,

        CAST(node__field_club_number.field_club_number_value AS SIGNED) AS club_number, 
    	node_field_data_paragraph__field_club.nid AS club_uid,
    	node_field_data_paragraph__field_club.title AS club_name,
        node_field__data_paragraph_field_club__field_region_number.field_region_number_value as club_region,
        node_field_data_node__field_region.nid AS club_region_uid,

        CAST(alldata.membership_expire AS DATE) as expiration_date,
        CAST(alldata.membership_join_year AS DATE) as join_date

    FROM
    	paragraphs_item_field_data
    	LEFT JOIN paragraph__field_club ON paragraphs_item_field_data.id = paragraph__field_club.entity_id
    		AND paragraph__field_club.deleted = '0'
    		AND(paragraph__field_club.langcode = paragraphs_item_field_data.langcode
    			OR paragraph__field_club.bundle = 'membership')
    		INNER JOIN node_field_data node_field_data_paragraph__field_club ON paragraph__field_club.field_club_target_id = node_field_data_paragraph__field_club.nid
    		LEFT JOIN node__field_region node_field_data_paragraph__field_club__node__field_region ON node_field_data_paragraph__field_club.nid = node_field_data_paragraph__field_club__node__field_region.entity_id
    			AND node_field_data_paragraph__field_club__node__field_region.deleted = '0'
    	LEFT JOIN node__field_club_number ON 
            node__field_club_number.entity_id = node_field_data_paragraph__field_club.nid
    	LEFT JOIN node_field_data node_field_data_node__field_region ON 
            node_field_data_paragraph__field_club__node__field_region.field_region_target_id = node_field_data_node__field_region.nid
		LEFT JOIN node__field_region_number node_field__data_paragraph_field_club__field_region_number ON
			node_field_data_paragraph__field_club__node__field_region.field_region_target_id = node_field__data_paragraph_field_club__field_region_number.entity_id
    	LEFT JOIN paragraph__field_leave_date paragraph__field_leave_date ON paragraphs_item_field_data.id = paragraph__field_leave_date.entity_id
    		AND paragraph__field_leave_date.deleted = '0'
    		AND(paragraph__field_leave_date.langcode = paragraphs_item_field_data.langcode
    			OR paragraph__field_leave_date.bundle = 'membership')
    	LEFT JOIN paragraph__field_join_date paragraph__field_join_date ON paragraphs_item_field_data.id = paragraph__field_join_date.entity_id
    		AND paragraph__field_join_date.deleted = '0'
    		AND(paragraph__field_join_date.langcode = paragraphs_item_field_data.langcode
    			OR paragraph__field_join_date.bundle = 'membership')
    		INNER JOIN users_field_data users_field_data ON paragraphs_item_field_data.parent_id = users_field_data.uid
    		LEFT JOIN user__field_primary_member user_is_primary_member ON users_field_data.uid = user_is_primary_member.entity_id
    		INNER JOIN z_member_search_main alldata ON users_field_data.uid = alldata.user_id
    		INNER JOIN ssp_membership_international_membership rightmembership ON users_field_data.uid = rightmembership.user_id
    		LEFT JOIN paragraph__field_membership_class memclass ON rightmembership.paragraph_id = memclass.entity_id
    		LEFT JOIN taxonomy_term_field_data memclassterm ON memclass.field_membership_class_target_id = memclassterm.tid
    		LEFT JOIN v_brns brns ON users_field_data.uid = brns.user_id
    		LEFT JOIN user__field_home_club userhomeclub ON paragraphs_item_field_data.id = userhomeclub.field_home_club_target_id
    			AND userhomeclub.deleted = '0'
    	LEFT JOIN user__field_memberships useraffclub ON paragraphs_item_field_data.id = useraffclub.field_memberships_target_id
    		AND useraffclub.deleted = '0'
    	LEFT JOIN user__field_intraclub_memberships userintraclub ON paragraphs_item_field_data.id = userintraclub.field_intraclub_memberships_target_id
    		AND userintraclub.deleted = '0'
    WHERE (((paragraphs_item_field_data.status = '1')
    		AND(paragraphs_item_field_data.type IN('membership'))
    		AND(paragraph__field_leave_date.field_leave_date_value IS NOT NULL)
    		AND(paragraph__field_join_date.field_join_date_value IS NOT NULL))
    	AND((alldata.personal_status_id IN('947'))
    	AND((CAST(paragraph__field_leave_date.field_leave_date_value AS DATE) >= NOW()))
    AND((CAST(paragraph__field_join_date.field_join_date_value AS DATE) <= NOW()))
    AND(((useraffclub.entity_id IS NOT NULL
    	OR userhomeclub.entity_id IS NOT NULL
    	OR userintraclub.entity_id IS NOT NULL)))))
    AND(user_is_primary_member.field_primary_member_target_id IS NULL)
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

#[derive(Debug, serde::Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MemberClass {
    #[default]
    Regular,
    Lifetime,
}

impl TryFrom<String> for MemberClass {
    type Error = sqlx::Error;
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "regular" => Ok(Self::Regular),
            "lifetime" => Ok(Self::Lifetime),
            other => Err(sqlx::Error::decode(format!(
                "unexpected member class {}",
                other
            ))),
        }
    }
}

#[derive(Debug, serde::Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MemberType {
    #[default]
    Regular,
    Affiliate,
}

impl TryFrom<String> for MemberType {
    type Error = sqlx::Error;
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "field_home_club" => Ok(Self::Regular),
            "field_memberships" => Ok(Self::Affiliate),
            other => Err(sqlx::Error::decode(format!(
                "unexpected member type{}",
                other
            ))),
        }
    }
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Member {
    #[sqlx(default, try_from = "String")]
    pub member_class: MemberClass,
    #[sqlx(default, try_from = "String")]
    pub member_type: MemberType,
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
    club_region_uid: Option<u64>,
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
