use crate::{cron::mailchimp::Job as MailchimpJob, Error};
use axum::{
    extract::State,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Router,
};
use axum_macros::FromRequest;
use itertools::Itertools;
use sqlx::PgPool;
use tokio_graceful_shutdown::SubsystemHandle;

pub async fn subsystem(
    addr: std::net::SocketAddr,
    db: PgPool,
    handle: SubsystemHandle,
) -> Result<(), Error> {
    let state = ApiState { db };
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tokio::select! {
        result = axum::serve(listener, router(state)) => result.map_err(Error::from),
        _ = handle.on_shutdown_requested() => Ok(())
    }
}

fn router(state: ApiState) -> Router {
    let api = Router::new().route("/sync", get(sync_list));

    Router::new().nest("/api/v1", api).with_state(state)
}

async fn sync_list(State(state): State<ApiState>) -> Result<ApiJson<Vec<MailchimpJob>>, ApiError> {
    let jobs = MailchimpJob::all(&state.db)
        .await?
        .into_iter()
        .map(|mut entry| {
            entry.api_key = "".to_string();
            entry
        })
        .collect_vec();
    Ok(ApiJson(jobs))
}

#[derive(Clone)]
pub struct ApiState {
    db: PgPool,
}

// Create our own JSON extractor by wrapping `axum::Json`. This makes it easy to override the
// rejection and provide our own which formats errors to match our application.
//
// `axum::Json` responds with plain text if the input is invalid.
#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(ApiError))]
pub struct ApiJson<T>(T);

impl<T> IntoResponse for ApiJson<T>
where
    axum::Json<T>: IntoResponse,
{
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

#[derive(Debug)]
pub enum ApiError {
    /// Resource not found
    NotFound(String),
    /// Internal error occurred
    Internal(String),
    /// Bad request supplied
    Request(String),
}

impl ApiError {
    pub fn invalid_request(msg: &str) -> Self {
        Self::Request(msg.to_string())
    }
}

impl From<anyhow::Error> for ApiError {
    fn from(value: anyhow::Error) -> Self {
        tracing::error!(?value, "service error");
        Self::Internal("internal service error".to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        #[derive(serde::Serialize)]
        struct ErrorResponse {
            message: String,
        }

        let (status, message) = match self {
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Self::Request(msg) => (StatusCode::BAD_REQUEST, msg),
        };
        (status, ApiJson(ErrorResponse { message })).into_response()
    }
}
