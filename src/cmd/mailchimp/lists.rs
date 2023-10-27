use crate::{
    cmd::{
        mailchimp::{read_toml, MergeFieldsConfig},
        print_json,
    },
    settings::Settings,
    Error, Result,
};
use futures::{TryFutureExt, TryStreamExt};
use sqlx::MySqlPool;
use std::sync::Arc;

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
    list_id: Option<String>,
}

impl List {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = mailchimp::client::from_api_key(&settings.mailchimp.api_key)?;
        if let Some(list_id) = &self.list_id {
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
    /// The name of a file with the list configuration
    config: String,
}

impl Create {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = mailchimp::client::from_api_key(&settings.mailchimp.api_key)?;
        let list_config: mailchimp::lists::List = read_toml(&self.config)?;
        let list = mailchimp::lists::create(&client, &list_config).await?;

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
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = mailchimp::client::from_api_key(&settings.mailchimp.api_key)?;
        mailchimp::lists::delete(&client, &self.list_id).await?;
        Ok(())
    }
}

/// Get information about an audience list.
#[derive(Debug, clap::Args)]
pub struct Info {
    /// The list ID of the list to get
    list_id: String,
}

impl Info {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = mailchimp::client::from_api_key(&settings.mailchimp.api_key)?;
        let list = mailchimp::lists::get(&client, &self.list_id).await?;
        print_json(&list)
    }
}

/// Udpate an audience to match a configuration file.
#[derive(Debug, clap::Args)]
pub struct Update {
    /// The name of a file with the list configuration
    config: String,
}

impl Update {
    pub async fn run(&self, settings: &Settings) -> Result {
        let client = mailchimp::client::from_api_key(&settings.mailchimp.api_key)?;
        let list_config: mailchimp::lists::List = read_toml(&self.config)?;
        let list = mailchimp::lists::update(&client, &list_config.id, &list_config).await?;

        print_json(&list)
    }
}

/// Sync the members or an audience from a database.
#[derive(Debug, clap::Args)]
pub struct Sync {
    /// The name of a file with the list configuration
    config: String,

    /// The merge field definition file
    fields: String,

    /// The email address of a single member to sync
    #[clap(long)]
    member: Option<String>,
}

impl Sync {
    pub async fn run(&self, settings: &Settings) -> Result {
        use futures::stream::StreamExt;
        let client = Arc::new(mailchimp::client::from_api_key(
            &settings.mailchimp.api_key,
        )?);
        let db = settings.database.connect().await?;
        let list_config: &mailchimp::lists::List = &read_toml(&self.config)?;
        let merge_fields: &mailchimp::merge_fields::MergeFields =
            &read_toml::<MergeFieldsConfig>(&self.fields)?.into();

        let stream: ddb::Stream<ddb::members::Member> = if let Some(email) = &self.member {
            let db_member = ddb::members::by_email(&db, email)
                .await?
                .ok_or(anyhow::anyhow!("Member not found: {email}"))?;
            futures::stream::once(async { Ok(db_member) }).boxed()
        } else {
            ddb::members::all(&db)
        };
        stream
            .map_err(Error::from)
            .map_ok(|member| (client.clone(), db.clone(), member))
            .try_for_each_concurrent(10, |(client, db, member)| async move {
                sync_member(&client, db, list_config, merge_fields, member).await
            })
            .await?;
        Ok(())
    }
}

async fn sync_member(
    client: &mailchimp::Client,
    db: MySqlPool,
    list_config: &mailchimp::lists::List,
    merge_fields: &mailchimp::merge_fields::MergeFields,
    member: ddb::members::Member,
) -> Result {
    let address = ddb::members::mailing_address_by_uid(&db, member.primary.uid).await?;
    let primary = to_member(&member, &address, &member.primary, merge_fields).await?;

    if let Some(parnter_user) = &member.partner {
        let mut partner = to_member(&member, &address, parnter_user, merge_fields).await?;
        if let Some(ref mut merge_fields) = partner.merge_fields {
            merge_fields.insert("PRIMARY".into(), member.primary.email.clone().into());
        }
        if mailchimp::members::is_valid_email(&partner.email_address) {
            // println!("Partner {}", partner.email_address);
            mailchimp::members::upsert(client, &list_config.id, &partner.email_address, &partner)
                .map_ok(|_| ())
                .or_else(|err| handle_mailchimp_error("partner", parnter_user, err))
                .await?;
        }
    }

    if mailchimp::members::is_valid_email(&primary.email_address) {
        // println!("Primary {}", primary.email_address);
        mailchimp::members::upsert(client, &list_config.id, &primary.email_address, &primary)
            .map_ok(|_| ())
            .or_else(|err| handle_mailchimp_error("primary", &member.primary, err))
            .await?;
    }

    Ok(())
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
        merge_fields.to_value("STATE", address.state.as_ref()),
        merge_fields.to_value("COUNTRY", address.country.as_ref()),
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
