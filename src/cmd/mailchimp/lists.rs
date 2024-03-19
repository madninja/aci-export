use crate::{
    cmd::print_json,
    settings::{read_merge_fields, read_toml, Settings},
    Error, Result,
};
use ddb::members::{MemberClass, MemberStatus, MemberType};
use futures::{stream::StreamExt, TryStreamExt};
use mailchimp::members::{MemberTagStatus, MemberTagUpdate, MEMBER_BATCH_UPSERT_MAX};
use serde_json::json;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::RwLock;

/// Commands on audience lists.
#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: ListsCommand,
}

impl Cmd {
    pub async fn run(&self, settings: &Settings) -> Result {
        self.cmd.run(settings).await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum ListsCommand {
    List(List),
    Create(Create),
    Delete(Delete),
    Info(Info),
    Update(Update),
    Sync(Sync),
}

impl ListsCommand {
    pub async fn run(&self, settings: &Settings) -> Result {
        match self {
            Self::List(cmd) => cmd.run(settings).await,
            Self::Create(cmd) => cmd.run(settings).await,
            Self::Delete(cmd) => cmd.run(settings).await,
            Self::Info(cmd) => cmd.run(settings).await,
            Self::Update(cmd) => cmd.run(settings).await,
            Self::Sync(cmd) => cmd.run(settings).await,
        }
    }
}

/// List all or a specific audience list.
#[derive(Debug, clap::Args)]
pub struct List {
    /// The list ID to get information for
    #[arg(long)]
    list: Option<String>,
}

impl List {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = settings.mailchimp.client()?;
        if let Some(list_id) = &self.list {
            let list = mailchimp::lists::get(&client, list_id).await?;
            print_json(&list)
        } else {
            let lists = mailchimp::lists::all(&client, Default::default())
                .try_collect::<Vec<_>>()
                .await?;
            print_json(&lists)
        }
    }
}

/// Create a new audience list.
#[derive(Debug, clap::Args)]
pub struct Create {
    /// Config file describing list to create
    descriptor: String,

    /// Merge fields to create
    merge_fields: Option<String>,
}

impl Create {
    pub async fn run(&self, settings: &Settings) -> Result {
        let list: mailchimp::lists::List = read_toml(&self.descriptor)?;
        let client = settings.mailchimp.client()?;
        let new_list = mailchimp::lists::create(&client, &list).await?;

        if let Some(fields_descriptor) = &self.merge_fields {
            let merge_fields = read_merge_fields(fields_descriptor)?;
            let _ = crate::cmd::mailchimp::merge_fields::update_merge_fields(
                &client,
                &new_list.id,
                merge_fields,
                true,
            )
            .await?;
        }

        print_json(&list)
    }
}

/// Delete an audience list.
#[derive(Debug, clap::Args)]
pub struct Delete {
    /// The list ID of the list to delete
    list: String,
}

impl Delete {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = settings.mailchimp.client()?;
        mailchimp::lists::delete(&client, &self.list).await?;
        Ok(())
    }
}

/// Get information about an audience list.
#[derive(Debug, clap::Args)]
pub struct Info {
    /// ID of the list to get
    list: Option<String>,
}

impl Info {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = settings.mailchimp.client()?;
        let list = settings.mailchimp.list_override(&self.list)?;
        let info = mailchimp::lists::get(&client, list).await?;
        print_json(&info)
    }
}

/// Udpate an audience to match a configuration file.
#[derive(Debug, clap::Args)]
pub struct Update {
    /// ID for list to update
    list: Option<String>,
    /// Descriptor file for list
    descriptor: String,
}

impl Update {
    pub async fn run(&self, settings: &Settings) -> Result {
        let descriptor: mailchimp::lists::List = read_toml(&self.descriptor)?;
        let list = settings.mailchimp.list_override(&self.list)?;
        let client = settings.mailchimp.client()?;
        let updated = mailchimp::lists::update(&client, list, &descriptor).await?;

        print_json(&updated)
    }
}

/// Sync a single emember, club members, or a members in a region to an audience
#[derive(Debug, clap::Args)]
pub struct Sync {
    /// List ID to sync settings with
    list: Option<String>,
    /// Merge fields dedescriptor file to use
    merge_fields: Option<String>,
    /// The email address of a single member to sync
    #[arg(long)]
    member: Option<String>,
    /// The ID of a club to sync
    #[arg(long)]
    club: Option<u64>,
    /// The ID of a region to sync
    #[arg(long)]
    region: Option<u64>,
}

impl Sync {
    pub async fn run(&self, settings: &Settings) -> Result {
        let merge_fields =
            read_merge_fields(settings.mailchimp.fields_override(&self.merge_fields)?)?;
        let list = settings.mailchimp.list_override(&self.list)?;
        let client = Arc::new(settings.mailchimp.client()?);
        let db = settings.database.connect().await?;

        let db_members = if let Some(email) = &self.member {
            vec![ddb::members::by_email(&db, email)
                .await?
                .ok_or(anyhow::anyhow!("Member not found: {email}"))?]
        } else if let Some(club) = settings.mailchimp.club_override(self.club) {
            ddb::members::by_club(&db, club).await?
        } else if let Some(region) = settings.mailchimp.region_override(self.region) {
            ddb::members::by_region(&db, region).await?
        } else {
            ddb::members::all(&db).await?
        };
        let upserted = Arc::new(RwLock::new(HashSet::new()));

        // Fetch addresses for primary members
        let db_addresses: HashMap<u64, ddb::members::Address> =
            ddb::members::mailing_address::by_uids(
                &db,
                &db_members
                    .iter()
                    .map(|member| member.primary.uid)
                    .collect::<Vec<u64>>(),
            )
            .await?;

        // Convert ddb members to mailchimp members while injecting address
        let mc_members: Vec<mailchimp::members::Member> = futures::stream::iter(db_members)
            .map(|member| (member, &merge_fields, &db_addresses))
            .map(|(member, merge_fields, addresses)| {
                let address = addresses.get(&member.primary.uid);
                match to_members(&member, &address.cloned(), merge_fields) {
                    Ok(members) => futures::stream::iter(members).map(Ok).boxed(),
                    Err(err) => futures::stream::once(async { Err(err) }).boxed(),
                }
            })
            .flatten()
            .try_collect::<Vec<_>>()
            .await?;

        // chunk in max sizes and yse batch_upsert to upsert the members in the list
        futures::stream::iter(mc_members)
            .chunks(MEMBER_BATCH_UPSERT_MAX)
            .map(Ok::<Vec<_>, Error>)
            .map_ok(|members| (client.clone(), members, upserted.clone()))
            .try_for_each_concurrent(10, |(client, members, processed)| async move {
                let response = mailchimp::members::batch_upsert(&client, list, &members).await?;
                let mut set = processed.write().await;
                response
                    .updated_members
                    .into_iter()
                    .chain(response.new_members)
                    .for_each(|entry| {
                        set.insert(entry.id);
                    });
                if response.error_count > 0 {
                    response.errors.iter().for_each(|err| {
                        println!("email: {} error: {}", err.email_address, err.error);
                    })
                }
                Ok(())
            })
            .await?;

        // Iterate through all mailchimp audience member. Collect all members that are not
        // the upserted set by set subtraction
        let mailchimp_stream = mailchimp::members::all(&client, list, Default::default());
        let audience: HashSet<String> = mailchimp_stream
            .map_err(Error::from)
            .try_filter_map(|member| async move {
                if member.status == Some(mailchimp::members::MemberStatus::Cleaned) {
                    Ok(None)
                } else {
                    Ok(Some(member.id))
                }
            })
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .collect();

        // don't process deletes for a single member sync
        let deleted = if self.member.is_none() {
            let to_delete = &audience - &*upserted.read().await;
            // Delete all to_delete entries
            futures::stream::iter(to_delete.iter())
                .map(|member_id| Ok::<_, crate::Error>((client.clone(), member_id)))
                .try_for_each_concurrent(10, |(client, member_id)| async move {
                    mailchimp::members::delete(&client, list, member_id).await?;
                    Ok(())
                })
                .await?;
            to_delete.len()
        } else {
            0
        };

        let json = json!({
                "upserted": upserted.read().await.len(),
                "deleted": deleted,
        });
        print_json(&json)
    }
}

fn to_members(
    member: &ddb::members::Member,
    address: &Option<ddb::members::Address>,
    merge_fields: &mailchimp::merge_fields::MergeFields,
) -> Result<Vec<mailchimp::members::Member>> {
    let primary = to_member(member, address, &member.primary, merge_fields)?;

    let mut result = Vec::with_capacity(2);
    if let Some(partner_user) = &member.partner {
        let mut partner = to_member(member, address, partner_user, merge_fields)?;
        if let Some(ref mut merge_fields) = partner.merge_fields {
            merge_fields.insert("PRIMARY".into(), member.primary.email.clone().into());
        }
        if mailchimp::members::is_valid_email(&partner.email_address) {
            result.push(partner);
        }
    }

    if mailchimp::members::is_valid_email(&primary.email_address) {
        result.push(primary);
    }

    Ok(result)
}

fn to_member(
    member: &ddb::members::Member,
    address: &Option<ddb::members::Address>,
    user: &ddb::users::User,
    merge_fields: &mailchimp::merge_fields::MergeFields,
) -> Result<mailchimp::members::Member> {
    let user_fields: Vec<mailchimp::merge_fields::MergeFieldValue> = [
        merge_fields.to_value("FNAME", user.first_name.as_ref()),
        merge_fields.to_value("LNAME", user.last_name.as_ref()),
        merge_fields.to_value("UID", user.uid),
        merge_fields.to_value("BDAY", user.birthday),
        merge_fields.to_value("JOIN", member.join_date),
        merge_fields.to_value("EXPIRE", member.expiration_date),
    ]
    .into_iter()
    .filter_map(|value| value.map_err(Error::from).transpose())
    .chain(address_to_values(address, merge_fields).into_iter())
    .chain(club_to_values(&member.local_club, merge_fields).into_iter())
    .collect::<Result<Vec<mailchimp::merge_fields::MergeFieldValue>>>()?;
    Ok(mailchimp::members::Member {
        id: mailchimp::members::member_id(&user.email),
        email_address: user.email.clone(),
        merge_fields: Some(user_fields.into_iter().collect()),
        status_if_new: Some(mailchimp::members::MemberStatus::Subscribed),
        ..Default::default()
    })
}

fn to_member_tag_updates(member: &ddb::members::Member) -> Vec<MemberTagUpdate> {
    fn to_update<F: Fn(&ddb::members::Member) -> bool>(
        name: &str,
        member: &ddb::members::Member,
        f: F,
    ) -> MemberTagUpdate {
        let status = if f(member) {
            MemberTagStatus::Active
        } else {
            MemberTagStatus::Inactive
        };
        MemberTagUpdate {
            name: name.to_string(),
            status,
        }
    }
    vec![
        to_update("affiliate", member, |m| {
            m.member_type == MemberType::Affiliate
        }),
        to_update("lifetime", member, |m| {
            m.member_class == MemberClass::Lifetime
        }),
        to_update("lapsed", member, |m| {
            m.member_status == MemberStatus::Lapsed
        }),
    ]
}

fn address_to_values(
    address: &Option<ddb::members::Address>,
    merge_fields: &mailchimp::merge_fields::MergeFields,
) -> Vec<Result<mailchimp::merge_fields::MergeFieldValue>> {
    let Some(address) = address.as_ref() else {
        return vec![];
    };

    vec![
        merge_fields.to_value("ZIP", address.zip_code.as_ref()),
        merge_fields.to_value("STATE", address.state.as_ref()),
        merge_fields.to_value("COUNTRY", address.country.as_ref()),
    ]
    .into_iter()
    .filter_map(|value| value.map_err(Error::from).transpose())
    .collect()
}

fn club_to_values(
    club: &ddb::clubs::Club,
    merge_fields: &mailchimp::merge_fields::MergeFields,
) -> Vec<Result<mailchimp::merge_fields::MergeFieldValue>> {
    vec![
        merge_fields.to_value("CLUB", club.name.as_str()),
        merge_fields.to_value("CLUB_NR", club.number),
        merge_fields.to_value("REGION", club.region as u64),
    ]
    .into_iter()
    .filter_map(|value| value.map_err(Error::from).transpose())
    .collect()
}
