use crate::{cmd::print_json, settings::Settings, Result};
use anyhow::anyhow;
use ddb::{Address, Member, User};

#[derive(Debug, clap::Args)]
pub struct Cmd {
    #[command(subcommand)]
    cmd: MemberCmd,
}

impl Cmd {
    pub async fn run(&self, settings: &Settings) -> Result {
        self.cmd.run(settings).await
    }
}

#[derive(Debug, clap::Subcommand)]
pub enum MemberCmd {
    Email(Email),
    Uid(Uid),
}

impl MemberCmd {
    pub async fn run(&self, settings: &Settings) -> Result {
        match self {
            Self::Email(cmd) => cmd.run(settings).await,
            Self::Uid(cmd) => cmd.run(settings).await,
        }
    }
}

#[derive(Debug, clap::Args)]
pub struct Email {
    pub email: String,
}

impl Email {
    pub async fn run(&self, settings: &Settings) -> Result {
        let db = settings.database.connect().await?;
        let mut report: MemberReport = Member::by_email(&db, &self.email)
            .await?
            .ok_or_else(|| anyhow!("Member {} not found", self.email))?
            .into();

        inflate_report(&mut report, &db).await?;
        print_json(&report)
    }
}

#[derive(Debug, clap::Args)]
pub struct Uid {
    pub uid: u64,
}

async fn inflate_report(report: &mut MemberReport, db: &sqlx::Pool<sqlx::MySql>) -> Result {
    let res = tokio::try_join!(
        Address::mailing_address_by_uid(db, report.primary.uid),
        Member::expiration_date_by_uid(db, report.primary.uid),
        Member::join_date_by_uid(db, report.primary.uid)
    );
    match res {
        Ok((mailing_address, expiration_date, join_date)) => {
            report.mailing_address = mailing_address;
            report.expiration_date = expiration_date;
            report.join_date = join_date;
            Ok(())
        }
        Err(err) => Err(err.into()),
    }
}

impl Uid {
    pub async fn run(&self, settings: &Settings) -> Result {
        let db = settings.database.connect().await?;
        let mut report: MemberReport = Member::by_uid(&db, self.uid)
            .await?
            .ok_or_else(|| anyhow!("Member {} not found", self.uid))?
            .into();

        inflate_report(&mut report, &db).await?;

        print_json(&report)
    }
}

#[derive(Debug, serde::Serialize)]
struct MemberReport {
    primary: User,
    partner: Option<User>,
    mailing_address: Option<Address>,
    expiration_date: Option<chrono::NaiveDate>,
    join_date: Option<chrono::NaiveDate>,
}

impl From<Member> for MemberReport {
    fn from(value: Member) -> Self {
        Self {
            primary: value.primary,
            partner: value.partner,
            mailing_address: None,
            expiration_date: None,
            join_date: None,
        }
    }
}
