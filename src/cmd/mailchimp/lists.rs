use crate::{
    cmd::print_json,
    settings::{MailchimpSetting, Settings},
    Error, Result,
};
use futures::{stream::StreamExt, TryFutureExt, TryStreamExt};
use mailchimp::{self, members::member_id};
use serde_json::json;
use sqlx::MySqlPool;
use std::{collections::HashSet, sync::Arc};
use tokio::sync::RwLock;

/// Commands on audience lists.
#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: ListsCommand,
}

impl Cmd {
    pub async fn run(&self, settings: &Settings, profile: &MailchimpSetting) -> Result {
        self.cmd.run(settings, profile).await
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
    pub async fn run(&self, settings: &Settings, profile: &MailchimpSetting) -> Result {
        match self {
            Self::List(cmd) => cmd.run(settings, profile).await,
            Self::Create(cmd) => cmd.run(settings, profile).await,
            Self::Delete(cmd) => cmd.run(settings, profile).await,
            Self::Info(cmd) => cmd.run(settings, profile).await,
            Self::Update(cmd) => cmd.run(settings, profile).await,
            Self::Sync(cmd) => cmd.run(settings, profile).await,
        }
    }
}

/// List all or a specific audience list.
#[derive(Debug, clap::Args)]
pub struct List {
    /// Override the list ID to get information for
    #[arg(long)]
    list: Option<String>,
    #[arg(long, conflicts_with = "list")]
    all: bool,
}

impl List {
    pub async fn run(&self, _settings: &Settings, profile: &MailchimpSetting) -> Result {
        let client = profile.client()?;
        if self.all {
            let lists = mailchimp::lists::all(&client, Default::default())
                .try_collect::<Vec<_>>()
                .await?;
            return print_json(&lists);
        }
        let list = mailchimp::lists::get(&client, profile.list_override(&self.list)?).await?;
        print_json(&list)
    }
}

/// Create a new audience list.
#[derive(Debug, clap::Args)]
pub struct Create {}

impl Create {
    pub async fn run(&self, _settings: &Settings, profile: &MailchimpSetting) -> Result {
        let client = profile.client()?;
        let list = mailchimp::lists::create(&client, &profile.config()?).await?;
        let _ = crate::cmd::mailchimp::merge_fields::update_merge_fields(
            &client,
            &list.id,
            profile.fields()?,
        )
        .await?;

        print_json(&list)
    }
}

/// Delete an audience list.
#[derive(Debug, clap::Args)]
pub struct Delete {
    /// The list ID of the list to delete
    list_id: String,
}

impl Delete {
    pub async fn run(&self, _settings: &Settings, profile: &MailchimpSetting) -> Result {
        let client = profile.client()?;
        mailchimp::lists::delete(&client, &self.list_id).await?;
        Ok(())
    }
}

/// Get information about an audience list.
#[derive(Debug, clap::Args)]
pub struct Info {
    /// Overridelist ID of the list to get
    list: Option<String>,
}

impl Info {
    pub async fn run(&self, _settings: &Settings, profile: &MailchimpSetting) -> Result {
        let client = profile.client()?;
        let list = mailchimp::lists::get(&client, profile.list_override(&self.list)?).await?;
        print_json(&list)
    }
}

/// Udpate an audience to match a configuration file.
#[derive(Debug, clap::Args)]
pub struct Update {}

impl Update {
    pub async fn run(&self, _settings: &Settings, profile: &MailchimpSetting) -> Result {
        let list_config = profile.config()?;
        let list =
            mailchimp::lists::update(&profile.client()?, &list_config.id, &list_config).await?;

        print_json(&list)
    }
}

/// Sync the members or an audience from a database.
#[derive(Debug, clap::Args)]
pub struct Sync {
    /// The email address of a single member to sync
    #[arg(long)]
    member: Option<String>,
}

impl Sync {
    pub async fn run(&self, settings: &Settings, profile: &MailchimpSetting) -> Result {
        let client = Arc::new(profile.client()?);
        let db = settings.database.connect().await?;
        let list_config = &profile.config()?;
        let merge_fields = &profile.fields()?;

        let stream: ddb::Stream<ddb::members::Member> = if let Some(email) = &self.member {
            let db_member = ddb::members::by_email(&db, email)
                .await?
                .ok_or(anyhow::anyhow!("Member not found: {email}"))?;
            futures::stream::once(async { Ok(db_member) }).boxed()
        } else if let Some(club) = profile.club {
            let members = ddb::members::by_club(&db, club).await?;
            futures::stream::iter(members).map(Ok).boxed()
        } else {
            ddb::members::all(&db)
        };
        let upserted = Arc::new(RwLock::new(HashSet::new()));

        // Upsert from db stream into mailchimp, insert processed entries into
        // a set to retain all ddb ids
        stream
            .map_err(Error::from)
            .map_ok(|member| (client.clone(), db.clone(), member, upserted.clone()))
            .try_for_each_concurrent(10, |(client, db, member, processed)| async move {
                let upserted =
                    upsert_member(&client, db, list_config, merge_fields, member).await?;
                let mut set = processed.write().await;
                upserted.into_iter().for_each(|entry| {
                    set.insert(entry);
                });
                Ok(())
            })
            .await?;

        // Iterate through all mailchimp audience member. Collect all members that are not
        // the upserted set by set subtraction
        let mailchimp_stream =
            mailchimp::members::all(&client, &list_config.id, Default::default());
        let audience: HashSet<String> = mailchimp_stream
            .map_err(Error::from)
            .map_ok(|member| member.id.clone())
            .try_collect::<Vec<_>>()
            .await?
            .into_iter()
            .collect();
        let to_delete = &audience - &*upserted.read().await;

        // don't process deletes for a single member sync
        if self.member.is_some() {
            // Delete all to_delete entries
            futures::stream::iter(to_delete.iter())
                .map(|member_id| Ok::<_, crate::Error>((client.clone(), member_id)))
                .try_for_each_concurrent(10, |(client, member_id)| async move {
                    mailchimp::members::delete(&client, &list_config.id, member_id).await?;
                    Ok(())
                })
                .await?;
        }

        let json = json!({
                "upserted": upserted.read().await.len(),
                "deleted": to_delete.len(),
        });
        print_json(&json)
    }
}

async fn upsert_member(
    client: &mailchimp::Client,
    db: MySqlPool,
    list_config: &mailchimp::lists::List,
    merge_fields: &mailchimp::merge_fields::MergeFields,
    member: ddb::members::Member,
) -> Result<Vec<String>> {
    let address = ddb::members::mailing_address_by_uid(&db, member.primary.uid).await?;
    let primary = to_member(&member, &address, &member.primary, merge_fields).await?;
    let mut processed = Vec::with_capacity(2);

    if let Some(parnter_user) = &member.partner {
        let mut partner = to_member(&member, &address, parnter_user, merge_fields).await?;
        if let Some(ref mut merge_fields) = partner.merge_fields {
            merge_fields.insert("PRIMARY".into(), member.primary.email.clone().into());
        }
        if mailchimp::members::is_valid_email(&partner.email_address) {
            let partner_id = member_id(&partner.email_address);
            // println!("Partner {}", partner.email_address);
            mailchimp::members::upsert(client, &list_config.id, &partner_id, &partner)
                .map_ok(|_| ())
                .or_else(|err| handle_mailchimp_error("partner", parnter_user, err))
                .await?;
            processed.push(partner_id);
        }
    }

    if mailchimp::members::is_valid_email(&primary.email_address) {
        let member_id = member_id(&primary.email_address);
        // println!("Primary {}", primary.email_address);
        mailchimp::members::upsert(client, &list_config.id, &member_id, &primary)
            .map_ok(|_| ())
            .or_else(|err| handle_mailchimp_error("primary", &member.primary, err))
            .await?;
        processed.push(member_id);
    }

    Ok(processed)
}

async fn to_member(
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

async fn handle_mailchimp_error(
    kind: &'static str,
    user: &ddb::users::User,
    err: mailchimp::Error,
) -> mailchimp::Result {
    match err {
        mailchimp::Error::Mailchimp(err) if err.status == 400 => {
            eprintln!(
                "Mailchimp {} email: {} error: {}",
                kind, user.email, err.detail
            );
            Ok(())
        }
        other => Err(other),
    }
}
