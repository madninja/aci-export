use crate::{
    paged_query_impl, paged_response_impl, query_default_impl, Client, Result, Stream, NO_QUERY,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub async fn all(client: &Client, query: BatchesQuery) -> Stream<BatchInfo> {
    client.fetch_stream::<BatchesQuery, BatchesResponse>("/3.0/batches", query)
}

pub async fn for_id(client: &Client, id: &str) -> Result<BatchInfo> {
    client.fetch(&format!("/3.0/batches/{id}",), NO_QUERY).await
}

#[derive(Serialize, Debug, Default)]
pub struct Batch {
    operations: Vec<BatchOperation>,
}

impl Batch {
    pub fn post<'a, T>(&'a mut self, path: &str, json: &T) -> Result<&'a mut BatchOperation>
    where
        T: Serialize + ?Sized,
    {
        self.operations
            .push(BatchOperation::new(BatchMethod::POST, path, json)?);
        Ok(self.operations.last_mut().unwrap())
    }
    pub fn patch<'a, T>(&'a mut self, path: &str, json: &T) -> Result<&'a BatchOperation>
    where
        T: Serialize + ?Sized,
    {
        self.operations
            .push(BatchOperation::new(BatchMethod::POST, path, json)?);
        Ok(self.operations.last_mut().unwrap())
    }

    pub fn put<'a, T>(&'a mut self, path: &str, json: &T) -> Result<&'a mut BatchOperation>
    where
        T: Serialize + ?Sized,
    {
        self.operations
            .push(BatchOperation::new(BatchMethod::PUT, path, json)?);
        Ok(self.operations.last_mut().unwrap())
    }

    pub fn delete<'a, T>(&'a mut self, path: &str, json: &T) -> Result<&'a mut BatchOperation>
    where
        T: Serialize + ?Sized,
    {
        self.operations
            .push(BatchOperation::new(BatchMethod::DELETE, path, json)?);
        Ok(self.operations.last_mut().unwrap())
    }

    pub async fn run(&self, client: &Client, to_completion: bool) -> Result<BatchInfo> {
        let mut info: BatchInfo = client.post("/3.0/batches", self).await?;
        while to_completion && info.status != BatchStatus::Finished {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            info = for_id(client, &info.id).await?;
        }
        Ok(info)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BatchOperation {
    pub method: BatchMethod,
    pub path: String,
    pub params: serde_json::Map<String, serde_json::Value>,
    pub body: String,
    pub operation_id: String,
}

impl BatchOperation {
    pub fn new<T: Serialize + ?Sized>(method: BatchMethod, path: &str, json: &T) -> Result<Self> {
        Ok(BatchOperation {
            method,
            path: path.to_string(),
            params: Default::default(),
            body: serde_json::to_string(json)?,
            operation_id: "".to_string(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum BatchMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BatchInfo {
    pub id: String,
    status: BatchStatus,
    total_operations: u16,
    finished_operations: u16,
    errored_operations: u16,
    submitted_at: DateTime<Utc>,
    completed_at: String,
    response_body_url: String,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BatchStatus {
    Preprocessing,
    Pending,
    Started,
    Finalizing,
    Finished,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BatchesQuery {
    pub fields: String,
    pub count: u32,
    pub offset: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct BatchesResponse {
    pub batches: Vec<BatchInfo>,
    pub total_items: u16,
}

query_default_impl!(BatchesQuery);
paged_query_impl!(BatchesQuery, &[]);
paged_response_impl!(BatchesResponse, batches, BatchInfo);
