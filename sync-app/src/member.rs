use crate::{club, user, Error, Result};
use futures::{stream, StreamExt, TryStreamExt};
use sqlx::{postgres::PgExecutor, Postgres, QueryBuilder};
use std::{collections::HashMap, fmt};

pub async fn all<'c, E>(exec: E) -> Result<Vec<Member>>
where
    E: PgExecutor<'c>,
{
    let all = fetch_members_query()
        .build_query_as::<Member>()
        .fetch_all(exec)
        .await?;
    Ok(dedupe_members(all))
}

pub async fn by_club<'c, E>(exec: E, uid: i64) -> Result<Vec<Member>>
where
    E: PgExecutor<'c>,
{
    let members = fetch_members_query()
        .push("WHERE local_club = ")
        .push_bind(uid)
        .build_query_as::<Member>()
        .fetch_all(exec)
        .await?;
    Ok(dedupe_members(members))
}

pub async fn by_region<'c, E>(exec: E, uid: i64) -> Result<Vec<Member>>
where
    E: PgExecutor<'c>,
{
    let all = fetch_members_query()
        .push("WHERE region.uid = ")
        .push_bind(uid)
        .build_query_as::<Member>()
        .fetch_all(exec)
        .await?;

    Ok(dedupe_members(all))
}

/// Remove affiliates in the given members list that are also regualr members
pub fn dedupe_members(members: Vec<Member>) -> Vec<Member> {
    let (regulars, mut affiliates): (Vec<Member>, Vec<Member>) = members
        .into_iter()
        .partition(|member| member.member_type != MemberType::Affiliate);
    let mut member_map: HashMap<String, Member> = regulars
        .into_iter()
        .map(|member| (member.primary.email.clone(), member))
        .collect();

    affiliates.retain(|affiliate| !member_map.contains_key(&affiliate.primary.email));
    affiliates.into_iter().for_each(|affiliate| {
        member_map.insert(affiliate.primary.email.clone(), affiliate);
    });
    member_map.into_values().collect()
}

pub async fn by_uid<'c, E>(exec: E, uid: i64) -> Result<Option<Member>>
where
    E: PgExecutor<'c>,
{
    let member = fetch_members_query()
        .push("WHERE primary_user = ")
        .push_bind(uid)
        .build_query_as::<Member>()
        .fetch_optional(exec)
        .await?;

    Ok(member)
}

pub async fn by_email<'c, E>(exec: E, email: &str) -> Result<Option<Member>>
where
    E: PgExecutor<'c>,
{
    let member = fetch_members_query()
        .push("WHERE email = ")
        .push_bind(email)
        .build_query_as::<Member>()
        .fetch_optional(exec)
        .await?;

    Ok(member)
}

const FETCH_MEMBERS_QUERY: &str = r#"
    SELECT
        uid,
        email,
        first_name,
        last_name,
        birthday,
        state,
        phone_mobile,
        phone_home,
        last_login,

        partner.uid as partner_uid,
        partner.email as partner_email,
        partner.first_name as partner_fist_name,
        partner.last_name as partner_last_name,
        partner.birthday as partner_birthday,
        partner.state as partner_state,
        partner.phone_mobile as partner_phone_mobile,
        partner.phone_home as partner_phone_home,
        partner.last_login as partner_last_login,

        member.member_class,
        member.member_type,
        member.expiration_date,
        member.join_date,

        club.uid as club_uid,
        club.name as club_name,
        club.number as club_number,

        region.number as club_region

    FROM
        members member
        LEFT JOIN users user ON user.email= member.primary_user
        LEFT JOIN users partner ON user.email = member.partner_user
        LEFT JOIN clubs club ON club.number = member.local_club
        LEFT JOIN regions region ON region.number = club.region
"#;

fn fetch_members_query<'builder>() -> sqlx::QueryBuilder<'builder, Postgres> {
    sqlx::QueryBuilder::new(FETCH_MEMBERS_QUERY)
}

pub async fn upsert_many<'c, E>(exec: E, members: &[Member]) -> Result<u64>
where
    E: PgExecutor<'c> + Copy,
{
    if members.is_empty() {
        return Ok(0);
    }

    let affected: Vec<u64> = stream::iter(members)
        .chunks(5000)
        .map(Ok)
        .and_then(|chunk| async move {
            let result = QueryBuilder::new(
                r#"INSERT INTO members(
                    primary_user,
                    partner_user,
                    member_class,
                    member_type,
                    expiration_date,
                    join_date,
                    local_club
                ) "#,
            )
            .push_values(&chunk, |mut b, member| {
                b.push_bind(&member.primary.email)
                    .push_bind(member.partner.as_ref().map(|user| &user.email))
                    .push_bind(&member.member_class)
                    .push_bind(&member.member_type)
                    .push_bind(member.expiration_date)
                    .push_bind(member.join_date)
                    .push_bind(member.local_club.number);
            })
            .push(
                r#"ON CONFLICT(primary_user) DO UPDATE SET
                partner_user = excluded.partner_user,
                member_class = excluded.member_class,
                member_type = excluded.member_type,
                expiration_date = excluded.expiration_date,
                join_date = excluded.join_date,
                local_club = excluded.local_club
            "#,
            )
            .build()
            .execute(exec)
            .await?;
            Ok::<u64, Error>(result.rows_affected())
        })
        .try_collect()
        .await?;
    Ok(affected.iter().sum())
}

pub async fn retain<'c, E>(exec: E, members: &[Member]) -> Result<u64>
where
    E: PgExecutor<'c>,
{
    if members.is_empty() {
        return Ok(0);
    }
    let mut builder =
        sqlx::QueryBuilder::new(r#" DELETE FROM members WHERE primary_user NOT IN ("#);
    let mut seperated = builder.separated(", ");
    for member in members {
        seperated.push_bind(&member.primary.email);
    }
    seperated.push_unseparated(") ");
    let result = builder.build().execute(exec).await?;
    Ok(result.rows_affected())
}
pub mod mailing_address {
    use super::*;

    pub async fn by_uid<'c, E>(exec: E, uid: i64) -> Result<Option<Address>>
    where
        E: PgExecutor<'c>,
    {
        let member = fetch_mailing_address_query()
            .push("WHERE user = ")
            .push_bind(uid)
            .build_query_as::<Address>()
            .fetch_optional(exec)
            .await?;
        Ok(member)
    }

    pub async fn by_uids<'c, I: IntoIterator<Item = i64>, E>(
        exec: E,
        uids: I,
    ) -> Result<HashMap<i64, Address>>
    where
        E: PgExecutor<'c>,
    {
        let mut builder = fetch_mailing_address_query();
        let mut seperated = builder.push("AND user IN (").separated(", ");
        for value in uids {
            seperated.push_bind(value);
        }
        seperated.push_unseparated(") ");
        let members: HashMap<i64, Address> = builder
            .build_query_as::<Address>()
            .fetch_all(exec)
            .await?
            .into_iter()
            .filter_map(|address| address.user_id.map(|user_id| (user_id, address)))
            .collect();
        Ok(members)
    }

    /// Get addresses for given members primary user ids
    pub async fn for_members<'c, E>(
        exec: E,
        members: impl IntoIterator<Item = &Member>,
    ) -> Result<HashMap<i64, Address>>
    where
        E: PgExecutor<'c>,
    {
        by_uids(exec, members.into_iter().map(|member| member.primary.uid)).await
    }

    pub async fn all<'c, E>(exec: E) -> Result<Vec<Address>>
    where
        E: PgExecutor<'c>,
    {
        let members = fetch_mailing_address_query()
            .build_query_as::<Address>()
            .fetch_all(exec)
            .await?;
        Ok(members)
    }

    fn fetch_mailing_address_query<'builder>() -> sqlx::QueryBuilder<'builder, Postgres> {
        sqlx::QueryBuilder::new(
            r#"
            SELECT
                user,
                street_address,
                street_address_2,
                zip_code,
                city,
                state,
                country
            FROM addresses
            "#,
        )
    }
}

#[derive(Debug, serde::Serialize, Default, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "member_class", rename_all = "lowercase")]
pub enum MemberClass {
    #[default]
    Regular,
    Lifetime,
}

impl fmt::Display for MemberClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Regular => f.write_str("regular"),
            Self::Lifetime => f.write_str("lifetime"),
        }
    }
}

impl TryFrom<String> for MemberClass {
    type Error = sqlx::Error;
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "regular" => Ok(Self::Regular),
            "lifetime" => Ok(Self::Lifetime),
            other => Err(sqlx::Error::decode(format!(
                "unexpected member class {other}"
            ))),
        }
    }
}

impl From<ddb::members::MemberClass> for MemberClass {
    fn from(value: ddb::members::MemberClass) -> Self {
        match value {
            ddb::members::MemberClass::Regular => Self::Regular,
            ddb::members::MemberClass::Lifetime => Self::Lifetime,
        }
    }
}

#[derive(Debug, serde::Serialize, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MemberStatus {
    #[default]
    Current,
    Lapsed,
}

impl fmt::Display for MemberStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Current => f.write_str("current"),
            Self::Lapsed => f.write_str("lapsed"),
        }
    }
}

impl TryFrom<String> for MemberStatus {
    type Error = sqlx::Error;
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "current" => Ok(Self::Current),
            "lapsed" => Ok(Self::Lapsed),
            other => Err(sqlx::Error::decode(format!(
                "unexpected member status {other}"
            ))),
        }
    }
}

impl TryFrom<i32> for MemberStatus {
    type Error = sqlx::Error;
    fn try_from(value: i32) -> std::result::Result<Self, Self::Error> {
        match value {
            947 => Ok(Self::Current),
            951 => Ok(Self::Lapsed),
            other => Err(sqlx::Error::decode(format!(
                "unexpected member status {other}"
            ))),
        }
    }
}

impl From<ddb::members::MemberStatus> for MemberStatus {
    fn from(value: ddb::members::MemberStatus) -> Self {
        match value {
            ddb::members::MemberStatus::Current => Self::Current,
            ddb::members::MemberStatus::Lapsed => Self::Lapsed,
        }
    }
}

#[derive(Debug, serde::Serialize, Default, PartialEq, Eq, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "member_type", rename_all = "lowercase")]
pub enum MemberType {
    #[default]
    Regular,
    Affiliate,
}

impl fmt::Display for MemberType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Regular => f.write_str("regular"),
            Self::Affiliate => f.write_str("affiliate"),
        }
    }
}

impl TryFrom<String> for MemberType {
    type Error = sqlx::Error;
    fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "field_home_club" | "regular" => Ok(Self::Regular),
            "field_memberships" | "affiliate" => Ok(Self::Affiliate),
            other => Err(sqlx::Error::decode(format!(
                "unexpected member type {}",
                other
            ))),
        }
    }
}

impl From<ddb::members::MemberType> for MemberType {
    fn from(value: ddb::members::MemberType) -> Self {
        match value {
            ddb::members::MemberType::Regular => Self::Regular,
            ddb::members::MemberType::Affiliate => Self::Affiliate,
        }
    }
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
pub struct Member {
    #[sqlx(default, try_from = "String")]
    pub member_class: MemberClass,
    #[sqlx(default, try_from = "String")]
    pub member_type: MemberType,
    #[sqlx(default, try_from = "i32")]
    pub member_status: MemberStatus,
    #[sqlx(flatten)]
    pub primary: user::User,
    #[sqlx(flatten, try_from = "PartnerUser")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partner: Option<user::User>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiration_date: Option<chrono::NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub join_date: Option<chrono::NaiveDate>,
    #[sqlx(flatten, try_from = "LocalClub")]
    pub local_club: club::Club,
}

impl From<ddb::members::Member> for Member {
    fn from(value: ddb::members::Member) -> Self {
        Self {
            member_class: value.member_class.into(),
            member_type: value.member_type.into(),
            member_status: value.member_status.into(),
            primary: value.primary.into(),
            partner: value.partner.map(Into::into),
            expiration_date: value.expiration_date,
            join_date: value.join_date,
            local_club: value.local_club.into(),
        }
    }
}

#[derive(Debug, sqlx::FromRow, serde::Serialize, Clone)]
pub struct Address {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<i64>,
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
    partner_uid: Option<i64>,
    partner_email: Option<String>,
    partner_first_name: Option<String>,
    partner_last_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    partner_birthday: Option<chrono::NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partner_phone_mobile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partner_phone_home: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    partner_last_login: Option<chrono::NaiveDate>,
}

impl From<PartnerUser> for Option<user::User> {
    fn from(value: PartnerUser) -> Option<user::User> {
        if let Some(uid) = value.partner_uid {
            Some(user::User {
                uid,
                email: value.partner_email.unwrap(),
                first_name: value.partner_first_name,
                last_name: value.partner_last_name,
                birthday: value.partner_birthday,
                phone_mobile: value.partner_phone_mobile,
                phone_home: value.partner_phone_home,
                last_login: value.partner_last_login,
            })
        } else {
            None
        }
    }
}

#[derive(Debug, sqlx::FromRow, serde::Serialize)]
struct LocalClub {
    club_name: Option<String>,
    club_uid: Option<i64>,
    club_number: Option<i64>,
    club_region: Option<i64>,
    club_region_uid: Option<i64>,
}

impl From<LocalClub> for club::Club {
    fn from(value: LocalClub) -> Self {
        Self {
            uid: value.club_uid.unwrap_or_default(),
            number: value.club_number,
            name: value.club_name.unwrap_or_default(),
            region: value.club_region.unwrap_or_default(),
        }
    }
}
