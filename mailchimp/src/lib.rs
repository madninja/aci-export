use futures::{
    future, stream, Future as StdFuture, FutureExt, Stream as StdStream, StreamExt, TryFutureExt,
};
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE},
    Method, RequestBuilder, Url,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{fmt::Debug, pin::Pin, str::FromStr, time::Duration};
use tokio_retry2::strategy::jitter;

/// A type alias for `Future` that may return `crate::error::Error`
pub type Future<T> = Pin<Box<dyn StdFuture<Output = Result<T>> + Send>>;

/// A type alias for `Stream` that may result in `crate::error::Error`
pub type Stream<T> = Pin<Box<dyn StdStream<Item = Result<T>> + Send>>;

mod error;

pub mod batches;
pub mod health;
pub mod lists;
pub mod members;
pub mod merge_fields;

pub use error::{Error, Result};

/// The default timeout for API requests
pub const DEFAULT_TIMEOUT: u64 = 20;
/// A utility constant to pass an empty query slice to the various client fetch
/// functions
pub const NO_QUERY: &[&str; 0] = &[""; 0];
/// Default number of items to return in a query
pub const DEFAULT_QUERY_COUNT: u32 = 1000;

#[derive(Debug, Clone)]
pub struct BasicAuth {
    auth_header: HeaderValue,
    endpoint: Url,
}

#[derive(Debug)]
struct DataCenter(String);

impl FromStr for DataCenter {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut parts = s.split('-');

        let _key = parts.next();
        let dc = parts.next();

        dc.map(|dc| Self(dc.to_string()))
            .ok_or(Error::MalformedAPIKey)
    }
}

#[derive(Debug, Clone)]
pub enum AuthMode {
    Basic(BasicAuth),
}

impl AuthMode {
    pub fn new_basic_auth(key: &str) -> Result<Self> {
        use base64::Engine;
        let encoded =
            base64::engine::general_purpose::STANDARD.encode(format!("username:{key}").as_bytes());
        let auth_header = HeaderValue::from_str(&format!("Basic {encoded}"))
            .map_err(|_| Error::MalformedAPIKey)?;

        let dc: DataCenter = key.parse()?;
        let url = format!("https://{}.api.mailchimp.com", dc.0);
        let endpoint = Url::parse(&url)?;

        Ok(Self::Basic(BasicAuth {
            auth_header,
            endpoint,
        }))
    }

    pub fn has_token(&self) -> bool {
        match self {
            Self::Basic(_) => true,
        }
    }

    pub fn to_endpoint_url(&self) -> Url {
        match self {
            Self::Basic(auth) => auth.endpoint.clone(),
        }
    }

    pub fn to_request_url(&self, path: &str) -> Result<Url> {
        let mut uri = path.to_string();

        // Make sure we have the leading "/".
        if !uri.starts_with('/') {
            uri = format!("/{uri}");
        }

        self.to_endpoint_url().join(&uri).map_err(Error::from)
    }

    pub fn to_authorization_header(&self) -> HeaderValue {
        match self {
            Self::Basic(auth) => auth.auth_header.clone(),
        }
    }
}

pub fn read_config<'de, T: serde::Deserialize<'de>, S>(source: S) -> Result<T>
where
    S: config::Source + Send + Sync + 'static,
{
    let config = config::Config::builder()
        .add_source(source)
        .build()
        .and_then(|config| config.try_deserialize())?;
    Ok(config)
}

#[derive(Clone, Debug)]
pub struct Client {
    auth: AuthMode,
    client: reqwest::Client,
}

pub mod client {
    use super::*;

    pub fn from_api_key(key: &str) -> Result<crate::Client> {
        let auth = crate::AuthMode::new_basic_auth(key)?;
        Ok(crate::Client::new(auth))
    }
}

impl Client {
    /// Create a new client using a given base URL and a default
    /// timeout. The library will use absoluate paths based on this
    /// base_url.
    pub fn new(auth: AuthMode) -> Self {
        Self::new_with_timeout(auth, DEFAULT_TIMEOUT)
    }

    /// Create a new client using a given base URL, and request
    /// timeout value.  The library will use absoluate paths based on
    /// the given base_url.
    pub fn new_with_timeout(auth: AuthMode, timeout: u64) -> Self {
        let client = reqwest::Client::builder()
            .gzip(true)
            .timeout(Duration::from_secs(timeout))
            .build()
            .unwrap();
        Self { auth, client }
    }

    fn request(&self, method: Method, path: &str) -> Result<RequestBuilder> {
        let url = self.auth.to_request_url(path)?;

        // Set the default headers.
        let mut headers = HeaderMap::new();
        headers.append(AUTHORIZATION, self.auth.to_authorization_header());
        headers.append(CONTENT_TYPE, HeaderValue::from_static("application/json"));

        Ok(self.client.request(method, url).headers(headers))
    }

    pub fn fetch<T, Q>(&self, path: &str, query: &Q) -> Future<T>
    where
        T: 'static + DeserializeOwned + Send,
        Q: Serialize + ?Sized,
    {
        match self.request(Method::GET, path) {
            Ok(builder) => builder
                .query(query)
                .send()
                .map_err(Error::from)
                .and_then(|response| {
                    let status = response.status();
                    if status.is_client_error() {
                        return response
                            .json::<error::MailchimError>()
                            .map_err(error::Error::from)
                            .and_then(|e| async move { Err(Error::mailchimp(e)) })
                            .boxed();
                    }
                    match response.error_for_status() {
                        Ok(result) => result
                            .bytes()
                            .map_err(Error::from)
                            .and_then(|bytes| async move {
                                // println!("{}", String::from_utf8_lossy(&bytes));
                                serde_json::from_slice(&bytes).map_err(error::Error::from)
                            })
                            .boxed(),
                        Err(e) => future::err(error::Error::from(e)).boxed(),
                    }
                })
                .boxed(),
            Err(e) => future::err(e).boxed(),
        }
    }

    pub fn fetch_stream<Q, R>(&self, path: &str, mut query: Q) -> Stream<R::Item>
    where
        R: PagedResponse + 'static,
        Q: PagedQuery + 'static + Serialize,
    {
        let client = self.clone();
        let path = path.to_string();

        self.fetch::<R, _>(&path, &query)
            .map_ok(move |data| {
                // let mut query = query.clone();
                query.inc_offset(data.len() as u32);
                stream::try_unfold(
                    (data, client, path, query),
                    |(mut data, client, path, mut query)| async move {
                        match data.pop() {
                            Some(entry) => Ok(Some((entry, (data, client, path, query)))),
                            None => {
                                let mut data = client.fetch::<R, _>(&path, &query).await?;
                                let data_len = data.len();
                                if data_len > 0 {
                                    query.inc_offset(data_len as u32);
                                    let entry = data.pop().unwrap();
                                    Ok(Some((entry, (data, client, path, query))))
                                } else {
                                    Ok(None)
                                }
                            }
                        }
                    },
                )
            })
            .try_flatten_stream()
            .boxed()
    }

    pub fn submit<T, R>(&self, method: Method, path: &str, json: &T) -> Future<R>
    where
        T: Serialize + ?Sized,
        R: 'static + DeserializeOwned + std::marker::Send,
    {
        match self.request(method, path) {
            Ok(builder) => builder
                .json(json)
                .send()
                .map_err(error::Error::from)
                .and_then(|response| {
                    let status = response.status();
                    if status.is_client_error() {
                        return response
                            .json::<error::MailchimError>()
                            .map_err(error::Error::from)
                            .and_then(|e| async move { Err(Error::mailchimp(e)) })
                            .boxed();
                    }
                    match response.error_for_status() {
                        Ok(result) => result
                            .bytes()
                            .map_err(error::Error::from)
                            .and_then(|bytes| async move {
                                if bytes.is_empty() {
                                    serde_json::from_str("null").map_err(error::Error::from)
                                } else {
                                    // println!("{}", String::from_utf8_lossy(&bytes));
                                    serde_json::from_slice(&bytes).map_err(error::Error::from)
                                }
                            })
                            .boxed(),
                        // Ok(result) => result.json().map_err(error::Error::from).boxed(),
                        Err(e) => future::err(error::Error::from(e)).boxed(),
                    }
                })
                .boxed(),
            Err(e) => future::err(e).boxed(),
        }
    }

    pub fn post<T, R>(&self, path: &str, json: &T) -> Future<R>
    where
        T: Serialize + ?Sized,
        R: 'static + DeserializeOwned + std::marker::Send,
    {
        self.submit(Method::POST, path, json)
    }

    pub fn patch<T, R>(&self, path: &str, json: &T) -> Future<R>
    where
        T: Serialize + ?Sized,
        R: 'static + DeserializeOwned + std::marker::Send,
    {
        self.submit(Method::PATCH, path, json)
    }

    pub fn put<T, R>(&self, path: &str, json: &T) -> Future<R>
    where
        T: Serialize + ?Sized,
        R: 'static + DeserializeOwned + std::marker::Send,
    {
        self.submit(Method::PUT, path, json)
    }

    pub fn delete(&self, path: &str) -> Future<()> {
        match self.request(Method::DELETE, path) {
            Ok(builder) => builder
                .send()
                .map_err(error::Error::from)
                .and_then(|response| match response.error_for_status() {
                    Ok(_) => future::ok(()).boxed(),
                    Err(e) => future::err(error::Error::from(e)).boxed(),
                })
                .boxed(),
            Err(e) => future::err(e).boxed(),
        }
    }
}

#[derive(Clone, Copy, Default)]
pub enum RetryPolicy {
    #[default]
    None,
    Retries(usize),
}

impl RetryPolicy {
    pub fn none() -> Self {
        Self::None
    }

    pub fn with_retries(retries: usize) -> Self {
        Self::Retries(retries)
    }
}

impl IntoIterator for RetryPolicy {
    type Item = Duration;
    type IntoIter = std::vec::IntoIter<Duration>;

    fn into_iter(self) -> Self::IntoIter {
        use tokio_retry2::strategy::ExponentialFactorBackoff;
        let retries = match self {
            Self::None => vec![],
            Self::Retries(retries) => ExponentialFactorBackoff::from_factor(2.)
                .max_delay_millis(5000)
                .map(jitter)
                .take(retries)
                .collect(),
        };
        retries.into_iter()
    }
}

pub trait PagedQuery: Clone + Send + Serialize + Sync {
    fn default_fields() -> &'static [&'static str];
    fn fields(&self) -> &str;
    fn set_fields(&mut self, fields: String);
    fn append_fields(&mut self, fields: &[&str]) {
        self.set_fields(format!("{},{}", self.fields(), fields.join(",")));
    }

    fn set_count(&mut self, count: u32);

    fn offset(&self) -> u32;
    fn set_offset(&mut self, offset: u32);

    fn inc_offset(&mut self, inc: u32) {
        self.set_offset(self.offset() + inc)
    }
}

pub trait PagedResponse: DeserializeOwned + Send + Sync + Debug {
    type Item: DeserializeOwned + Send + Sync + Debug;

    fn pop(&mut self) -> Option<Self::Item>;
    fn len(&self) -> usize;
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

macro_rules! paged_query_impl {
    ($query_type:ident, $default_fields:expr) => {
        impl crate::PagedQuery for $query_type {
            fn default_fields() -> &'static [&'static str] {
                $default_fields
            }

            fn fields(&self) -> &str {
                &self.fields
            }

            fn set_fields(&mut self, fields: String) {
                self.fields = fields;
            }

            fn set_count(&mut self, count: u32) {
                self.count = count;
            }

            fn offset(&self) -> u32 {
                self.offset
            }

            fn set_offset(&mut self, offset: u32) {
                self.offset = offset;
            }
        }
    };
}

macro_rules! paged_response_impl {
    ($response_type:ident, $item_field:ident, $item_type:ident) => {
        impl crate::PagedResponse for $response_type {
            type Item = $item_type;

            fn pop(&mut self) -> Option<$item_type> {
                self.$item_field.pop()
            }
            fn len(&self) -> usize {
                self.$item_field.len()
            }
        }
    };
}

macro_rules! query_default_impl {
    ($query_type:ident) => {
        impl Default for $query_type {
            fn default() -> Self {
                use crate::PagedQuery;
                Self {
                    fields: Self::default_fields().join(","),
                    count: crate::DEFAULT_QUERY_COUNT,
                    offset: 0,
                }
            }
        }
    };
}

pub(crate) use {paged_query_impl, paged_response_impl, query_default_impl};

pub mod deserialize_null_string {
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer).unwrap_or_default();

        Ok(s)
    }
}

pub fn is_default<T>(value: &T) -> bool
where
    T: PartialEq + Default,
{
    *value == T::default()
}

pub mod deserialize_null_i32 {
    use super::I32Visitor;
    use serde::Deserializer;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<i32, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = deserializer.deserialize_i32(I32Visitor).unwrap_or_default();

        Ok(s)
    }
}

struct I32Visitor;

impl serde::de::Visitor<'_> for I32Visitor {
    type Value = i32;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("an integer between -2^31 and 2^31")
    }

    fn visit_i8<E>(self, value: i8) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(value as i32)
    }

    fn visit_i16<E>(self, value: i16) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(value as i32)
    }

    fn visit_i32<E>(self, value: i32) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(value)
    }

    fn visit_i64<E>(self, value: i64) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if value >= i64::from(i32::MIN) && value <= i64::from(i32::MAX) {
            Ok(value as i32)
        } else {
            Err(E::custom(format!("i32 out of range: {value}")))
        }
    }

    fn visit_u64<E>(self, value: u64) -> std::result::Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(value as i32)
    }
}
