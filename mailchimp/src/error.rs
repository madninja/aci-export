use thiserror::Error;
pub type Result<T = ()> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("malformed api key")]
    MalformedAPIKey,
    #[error("malformed url")]
    MalformedUrl(#[from] url::ParseError),
    #[error("request error: {0}")]
    Request(#[from] reqwest::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("mailchimp error {}: {}", .0.status, .0.detail)]
    Mailchimp(MailchimError),
    #[error("unexpected value: {0}")]
    Value(serde_json::Value),
    #[error("unexpected or invalid number: {0}")]
    Number(String),
    #[error("invalid merge type: {0}")]
    InvalidMergeType(String),
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct MailchimError {
    pub status: u16,
    pub r#type: Option<String>,
    pub title: String,
    pub detail: String,
    pub instance: String,
}

impl Error {
    pub fn value(value: serde_json::Value) -> Self {
        Self::Value(value)
    }

    pub fn number(value: &str) -> Self {
        Self::Number(value.to_string())
    }

    pub fn mailchimp(value: MailchimError) -> Self {
        Self::Mailchimp(value)
    }
}
