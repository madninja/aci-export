use futures::{
    future, stream, Future as StdFuture, FutureExt, Stream as StdStream, StreamExt, TryFutureExt,
};
use reqwest::{
    header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE},
    Method, RequestBuilder, Url,
};
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fmt::Debug,
    marker::{Send, Sync},
    pin::Pin,
    str::FromStr,
    time::Duration,
};

/// A type alias for `Future` that may return `crate::error::Error`
pub type Future<T> = Pin<Box<dyn StdFuture<Output = Result<T>> + Send>>;

/// A type alias for `Stream` that may result in `crate::error::Error`
pub type Stream<T> = Pin<Box<dyn StdStream<Item = Result<T>> + Send>>;

mod error;
pub mod health;
pub mod lists;

pub use error::{Error, Result};

/// The default timeout for API requests
pub const DEFAULT_TIMEOUT: u64 = 10;
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
                .and_then(|response| match response.error_for_status() {
                    Ok(result) => {
                        let data: Future<T> = result.json().map_err(error::Error::from).boxed();
                        data
                    }
                    Err(e) => future::err(Error::from(e)).boxed(),
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

    pub fn post<T, R>(&self, path: &str, json: &T) -> Future<R>
    where
        T: Serialize + ?Sized,
        R: 'static + DeserializeOwned + std::marker::Send,
    {
        match self.request(Method::POST, path) {
            Ok(builder) => builder
                .json(json)
                .send()
                .map_err(error::Error::from)
                .and_then(|response| match response.error_for_status() {
                    Ok(result) => {
                        let data: Future<R> = result.json().map_err(error::Error::from).boxed();
                        data
                    }
                    Err(e) => future::err(error::Error::from(e)).boxed(),
                })
                .boxed(),
            Err(e) => future::err(e).boxed(),
        }
    }
}

pub trait PagedQuery: Clone + Send + Serialize + Sync {
    fn default_fields() -> &'static [&'static str];
    fn set_count(&mut self, count: u32);
    fn offset(&self) -> u32;
    fn set_offset(&mut self, offset: u32);

    fn default_fields_string() -> String {
        Self::default_fields().join(",")
    }
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

pub mod deserialize_null_string {
    use serde::{self, Deserialize, Deserializer};

    pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer).unwrap_or_default();

        Ok(s)
    }
}

#[cfg(test)]
fn get_test_client() -> Client {
    use std::{env, thread, time};
    const USER_AGENT: &str = "helium-api-test/0.1.0";
    const BASE_URL: &str = "https://api.helium.io/v1";
    let duration = time::Duration::from_millis(env::var("TEST_DELAY_MS").map_or(0, |v| {
        v.parse::<u64>()
            .expect("TEST_DELAY_MS cannot be parsed as u64")
    }));
    thread::sleep(duration);
    Client::new_with_base_url(BASE_URL.into(), USER_AGENT)
}
