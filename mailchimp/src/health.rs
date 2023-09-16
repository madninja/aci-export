use crate::{deserialize_null_string, Client, Result, NO_QUERY};
use serde::{Deserialize, Serialize};

pub async fn ping(client: &Client) -> Result<ApiHealthStatus> {
    client.fetch("/3.0/ping", NO_QUERY).await
}

/// API health status.
#[derive(Serialize, Deserialize, PartialEq, Debug, Clone)]
pub struct ApiHealthStatus {
    /**
     * API health status.
     */
    #[serde(
        default,
        skip_serializing_if = "String::is_empty",
        deserialize_with = "deserialize_null_string::deserialize"
    )]
    pub health_status: String,
}
