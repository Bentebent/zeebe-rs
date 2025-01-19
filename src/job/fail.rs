use crate::{proto, Client, ClientError};
use serde::Serialize;
use thiserror::Error;

/// Errors that can occur during job failure handling
#[derive(Error, Debug)]
pub enum FailJobError {
    /// Error that occurred during JSON serialization/deserialization
    #[error(transparent)]
    JsonError(#[from] serde_json::Error),
}

/// Initial state for the FailJobRequest builder pattern
pub struct Initial;

/// State indicating the job key has been set
pub struct WithKey;

/// Marker trait for FailJobRequest states
pub trait FailJobRequestState {}
impl FailJobRequestState for Initial {}
impl FailJobRequestState for WithKey {}

/// Request to mark a job as failed
#[derive(Debug, Clone)]
pub struct FailJobRequest<T: FailJobRequestState> {
    client: Client,
    /// The unique key identifying the job
    job_key: i64,
    /// Number of remaining retries for the job
    retries: i32,
    /// Message describing why the job failed
    error_message: String,
    /// Time to wait before retrying the job (in milliseconds)
    retry_back_off: i64,
    /// Variables to be set when failing the job
    variables: serde_json::Value,
    _state: std::marker::PhantomData<T>,
}

impl<T: FailJobRequestState> FailJobRequest<T> {
    /// Creates a new FailJobRequest in its initial state
    pub(crate) fn new(client: Client) -> FailJobRequest<Initial> {
        FailJobRequest {
            client,
            job_key: 0,
            retries: 0,
            error_message: String::new(),
            retry_back_off: 0,
            variables: serde_json::Value::default(),
            _state: std::marker::PhantomData,
        }
    }

    /// Internal helper to transition between builder states
    fn transition<NewState: FailJobRequestState>(self) -> FailJobRequest<NewState> {
        FailJobRequest {
            client: self.client,
            job_key: self.job_key,
            retries: self.retries,
            error_message: self.error_message,
            retry_back_off: self.retry_back_off,
            variables: self.variables,
            _state: std::marker::PhantomData,
        }
    }
}

impl FailJobRequest<Initial> {
    /// Sets the job key to identify which job failed
    pub fn with_job_key(mut self, job_key: i64) -> FailJobRequest<WithKey> {
        self.job_key = job_key;
        self.transition()
    }
}

impl FailJobRequest<WithKey> {
    /// Sets the number of remaining retries for the job
    pub fn with_retries(mut self, retries: i32) -> Self {
        self.retries = retries;
        self
    }

    /// Sets an error message describing why the job failed
    pub fn with_error_message(mut self, error_message: String) -> Self {
        self.error_message = error_message;
        self
    }

    /// Sets the time to wait before retrying the job
    ///
    /// # Arguments
    /// * `retry_back_off_sec` - Time to wait in seconds before retrying
    pub fn with_retry_back_off(mut self, retry_back_off_sec: i64) -> Self {
        self.retry_back_off = retry_back_off_sec * 1000;
        self
    }

    /// Sets variables to be included with the job failure
    pub fn with_variables<T: Serialize>(mut self, data: T) -> Result<Self, FailJobError> {
        self.variables = serde_json::to_value(data)?;
        Ok(self)
    }

    /// Sends the job failure request to the Zeebe workflow engine
    pub async fn send(mut self) -> Result<FailJobResponse, ClientError> {
        let res = self
            .client
            .gateway_client
            .fail_job(proto::FailJobRequest {
                job_key: self.job_key,
                retries: self.retries,
                error_message: self.error_message,
                retry_back_off: self.retry_back_off,
                variables: self.variables.to_string(),
            })
            .await?;

        Ok(res.into_inner().into())
    }
}

/// Response from marking a job as failed
#[derive(Debug, Clone)]
pub struct FailJobResponse {}

impl From<proto::FailJobResponse> for FailJobResponse {
    fn from(_value: proto::FailJobResponse) -> FailJobResponse {
        FailJobResponse {}
    }
}
