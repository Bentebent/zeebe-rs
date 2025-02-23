use crate::{Client, ClientError, proto};
use serde::Serialize;
use std::time::Duration;

pub struct Initial;
pub struct WithKey;

pub trait FailJobRequestState {}
impl FailJobRequestState for Initial {}
impl FailJobRequestState for WithKey {}

/// Request to mark a job as failed
///
/// This struct uses a builder pattern with state transitions to ensure that
/// required fields are set before sending the request. The state transitions
/// are enforced at compile time using marker traits.
///
/// # Examples
/// ```ignore
/// client
///     .fail_job()
///     .with_job_key(123456)
///     .send()
///     .await?;
/// ```
#[derive(Debug, Clone)]
pub struct FailJobRequest<T: FailJobRequestState> {
    client: Client,
    job_key: i64,
    retries: i32,
    error_message: String,
    retry_back_off: i64,
    variables: serde_json::Value,
    _state: std::marker::PhantomData<T>,
}

impl<T: FailJobRequestState> FailJobRequest<T> {
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
    ///
    /// # Arguments
    ///
    /// * `job_key` - The unique key identifying the job
    ///
    /// # Returns
    ///
    /// A FailJobRequest in the `WithKey` state
    pub fn with_job_key(mut self, job_key: i64) -> FailJobRequest<WithKey> {
        self.job_key = job_key;
        self.transition()
    }
}

impl FailJobRequest<WithKey> {
    /// Sets the number of remaining retries for the job
    ///
    /// # Arguments
    ///
    /// * `retries` - The number of remaining retries for the job
    ///
    /// # Returns
    ///
    /// The updated FailJobRequest
    pub fn with_retries(mut self, retries: i32) -> Self {
        self.retries = retries;
        self
    }

    /// Sets an error message describing why the job failed
    ///
    /// # Arguments
    ///
    /// * `error_message` - A message describing why the job failed
    ///
    /// # Returns
    ///
    /// The updated FailJobRequest
    pub fn with_error_message(mut self, error_message: String) -> Self {
        self.error_message = error_message;
        self
    }

    /// Sets the time to wait before retrying the job
    ///
    /// # Arguments
    ///
    /// * `retry_back_off` - Time to wait in seconds before retrying
    ///
    /// # Returns
    ///
    /// The updated FailJobRequest
    pub fn with_retry_back_off(mut self, retry_back_off: Duration) -> Self {
        self.retry_back_off = retry_back_off.as_millis() as i64;
        self
    }

    /// Sets variables to be included with the job failure
    ///
    /// # Arguments
    ///
    /// * `data` - The variables to be included with the job failure
    ///
    /// # Returns
    ///
    /// The updated FailJobRequest or a ClientError if serialization fails
    pub fn with_variables<T: Serialize>(mut self, data: T) -> Result<Self, ClientError> {
        self.variables = serde_json::to_value(data)
            .map_err(|e| ClientError::SerializationFailed { source: e })?;
        Ok(self)
    }

    /// Sends the job failure request to the Zeebe workflow engine
    ///
    /// # Returns
    ///
    /// A Result containing a FailJobResponse if successful, or a ClientError if the request fails
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
